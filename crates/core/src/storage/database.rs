//! 数据库连接、版本与事务（计划 §3.4）。
//!
//! - 读写连接由维护入口独占；查询使用独立只读连接。
//! - 每个组合查询在一个独立只读连接的读事务内完成。
//! - migration 只在维护连接上执行；新库被旧客户端只读打开时返回版本错误。

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::contract::error::{CoreError, codes};

/// 数据库 schema 版本。
pub const SCHEMA_VERSION: i64 = 3;

/// migration SQL（编译期内嵌）。
pub const MIGRATION_0001: &str = include_str!("../../migrations/0001_initial.sql");
pub const MIGRATION_0002: &str = include_str!("../../migrations/0002_candidates.sql");
pub const MIGRATION_0003: &str = include_str!("../../migrations/0003_knowledge.sql");

/// 错误码：数据库版本不受支持。
pub const DB_VERSION_UNSUPPORTED: &str = "DB_VERSION_UNSUPPORTED";

/// 数据库连接。
pub struct Database {
    conn: Connection,
    path: PathBuf,
    writable: bool,
}

impl Database {
    /// 打开只读连接。库不存在直接报错，不创建、不迁移。
    pub fn open_readonly(path: &Path) -> Result<Self, CoreError> {
        if !path.exists() {
            return Err(CoreError::new(
                codes::INDEX_MISSING,
                format!("index database `{}` does not exist", path.display()),
            ));
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            CoreError::new(
                codes::INDEX_MISSING,
                format!("cannot open index database `{}`: {error}", path.display()),
            )
        })?;
        conn.busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| {
                CoreError::new(
                    codes::BUSY,
                    format!("cannot configure busy timeout: {error}"),
                )
            })?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot enable foreign keys: {error}"),
                )
            })?;
        conn.pragma_update(None, "query_only", "ON")
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot enable query_only: {error}"),
                )
            })?;
        let database = Database {
            conn,
            path: path.to_path_buf(),
            writable: false,
        };
        database.check_schema_version()?;
        Ok(database)
    }

    /// 打开维护连接：不存在则创建，随后执行 migration。
    pub fn open_maintenance(
        path: &Path,
        project_id: &str,
        config_hash: &str,
        now: &str,
    ) -> Result<Self, CoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!(
                        "cannot create index directory `{}`: {error}",
                        parent.display()
                    ),
                )
            })?;
        }
        let conn = Connection::open(path).map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot open index database `{}`: {error}", path.display()),
            )
        })?;
        conn.busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| {
                CoreError::new(
                    codes::BUSY,
                    format!("cannot configure busy timeout: {error}"),
                )
            })?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot enable foreign keys: {error}"),
                )
            })?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| {
                CoreError::new(codes::INTERNAL_ERROR, format!("cannot enable WAL: {error}"))
            })?;
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot set synchronous=FULL: {error}"),
                )
            })?;

        let mut database = Database {
            conn,
            path: path.to_path_buf(),
            writable: true,
        };
        database.migrate(project_id, config_hash, now)?;
        Ok(database)
    }

    /// 底层连接。
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub(crate) fn query_connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// 只观察未展开参数的 SQL 文本，供宿主诊断。
    pub fn observe_sql(
        &mut self,
        mut observer: impl FnMut(&str) + Send + 'static,
    ) -> Result<(), CoreError> {
        self.conn
            .trace_v2(
                rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                Some(move |event: rusqlite::trace::TraceEvent<'_>| {
                    if let rusqlite::trace::TraceEvent::Stmt(statement, _) = event {
                        observer(&statement.sql());
                    }
                }),
            )
            .map_err(super::database_error)
    }

    /// 数据库路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 是否可写。
    pub fn is_writable(&self) -> bool {
        self.writable
    }

    /// 执行 migration 并确保 `index_meta` 存在。
    fn migrate(&mut self, project_id: &str, config_hash: &str, now: &str) -> Result<(), CoreError> {
        let applied: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if applied > SCHEMA_VERSION {
            return Err(CoreError::new(
                DB_VERSION_UNSUPPORTED,
                format!(
                    "index database schema version {applied} is newer than supported version {SCHEMA_VERSION}"
                ),
            ));
        }
        for (version, sql) in [
            (1, MIGRATION_0001),
            (2, MIGRATION_0002),
            (3, MIGRATION_0003),
        ] {
            if applied >= version {
                continue;
            }
            let checksum = crate::contract::identity::content_hash(sql.as_bytes());
            let transaction = self.conn.transaction().map_err(|error| {
                CoreError::new(
                    codes::BUSY,
                    format!("cannot begin migration transaction: {error}"),
                )
            })?;
            transaction.execute_batch(sql).map_err(|error| {
                CoreError::new(codes::INTERNAL_ERROR, format!("migration failed: {error}"))
            })?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![version, checksum, now],
                )
                .map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot record migration: {error}"),
                    )
                })?;
            transaction.commit().map_err(|error| {
                CoreError::new(codes::BUSY, format!("cannot commit migration: {error}"))
            })?;
        }

        let index_id: Option<String> = self
            .conn
            .query_row("SELECT index_id FROM index_meta WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read index_meta: {error}"),
                )
            })?;
        if index_id.is_none() {
            let index_id = uuid::Uuid::new_v4().to_string();
            self.conn
                .execute(
                    "INSERT INTO index_meta (id, index_id, project_id, generation, current_source_snapshot_id, config_hash, created_at)
                     VALUES (1, ?1, ?2, 0, NULL, ?3, ?4)",
                    rusqlite::params![index_id, project_id, config_hash, now],
                )
                .map_err(|error| {
                    CoreError::new(
                        codes::INTERNAL_ERROR,
                        format!("cannot initialize index_meta: {error}"),
                    )
                })?;
        }
        Ok(())
    }

    /// 校验 schema 版本（只读连接使用）。
    fn check_schema_version(&self) -> Result<(), CoreError> {
        let applied: Option<i64> = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read schema version: {error}"),
                )
            })?;
        match applied {
            None => Err(CoreError::new(
                DB_VERSION_UNSUPPORTED,
                "index database has no schema migration record",
            )),
            Some(version) if version != SCHEMA_VERSION => Err(CoreError::new(
                DB_VERSION_UNSUPPORTED,
                format!(
                    "index database schema version {version} differs from supported version {SCHEMA_VERSION}; use a maintenance command to migrate"
                ),
            )),
            Some(_) => Ok(()),
        }
    }

    /// 索引 ID。
    pub fn index_id(&self) -> Result<String, CoreError> {
        self.conn
            .query_row("SELECT index_id FROM index_meta WHERE id = 1", [], |row| {
                row.get(0)
            })
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read index id: {error}"),
                )
            })
    }

    /// 项目 ID。
    pub fn project_id(&self) -> Result<String, CoreError> {
        self.conn
            .query_row(
                "SELECT project_id FROM index_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read project id: {error}"),
                )
            })
    }

    /// 当前 generation（内部整数形式）。
    pub fn generation(&self) -> Result<u64, CoreError> {
        let value: i64 = self
            .conn
            .query_row(
                "SELECT generation FROM index_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read generation: {error}"),
                )
            })?;
        u64::try_from(value).map_err(|_| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("generation {value} is out of range"),
            )
        })
    }

    /// generation 的字符串形式（API 使用无符号十进制字符串）。
    pub fn generation_string(&self) -> Result<String, CoreError> {
        Ok(self.generation()?.to_string())
    }

    /// 配置哈希。
    pub fn config_hash(&self) -> Result<String, CoreError> {
        self.conn
            .query_row(
                "SELECT config_hash FROM index_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read config hash: {error}"),
                )
            })
    }

    /// 当前来源快照 ID。
    pub fn current_source_snapshot_id(&self) -> Result<Option<String>, CoreError> {
        self.conn
            .query_row(
                "SELECT current_source_snapshot_id FROM index_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read current snapshot id: {error}"),
                )
            })
    }

    /// generation +1（失败调用不会推进）。
    pub fn bump_generation(&self) -> Result<u64, CoreError> {
        self.conn
            .execute(
                "UPDATE index_meta SET generation = generation + 1 WHERE id = 1",
                [],
            )
            .map_err(|error| {
                CoreError::new(codes::BUSY, format!("cannot bump generation: {error}"))
            })?;
        self.generation()
    }

    /// 在只读连接上开启读事务。
    pub fn read_transaction(&mut self) -> Result<rusqlite::Transaction<'_>, CoreError> {
        self.conn.unchecked_transaction().map_err(|error| {
            CoreError::new(
                codes::BUSY,
                format!("cannot begin read transaction: {error}"),
            )
        })
    }

    /// 写入事务：`BEGIN IMMEDIATE`，脚本/模型/文件准备必须在事务外完成。
    pub fn write_transaction(&mut self) -> Result<rusqlite::Transaction<'_>, CoreError> {
        self.conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| {
                CoreError::new(
                    codes::BUSY,
                    format!("cannot begin write transaction: {error}"),
                )
            })
    }

    /// 读取 SQLite 版本与 FTS5/JSON 支持（能力探测）。
    pub fn capabilities(&self) -> Result<SqliteCapabilities, CoreError> {
        let sqlite_version: String = self
            .conn
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read sqlite version: {error}"),
                )
            })?;
        let fts5: bool = self
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='search_fts'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false);
        let json: bool = self
            .conn
            .query_row("SELECT json_valid('{}')", [], |row| row.get::<_, i64>(0))
            .map(|value| value == 1)
            .unwrap_or(false);
        let foreign_keys: bool = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .map(|value| value == 1)
            .unwrap_or(false);
        Ok(SqliteCapabilities {
            sqlite_version,
            fts5,
            json,
            foreign_keys,
        })
    }
}

/// SQLite 能力探测结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteCapabilities {
    pub sqlite_version: String,
    pub fts5: bool,
    pub json: bool,
    pub foreign_keys: bool,
}

/// 使用 backup API 制作一致快照（不能只复制活跃 WAL 数据库主文件）。
pub fn backup_to(source: &Path, destination: &Path) -> Result<(), CoreError> {
    let source_connection = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot open source database for backup: {error}"),
        )
    })?;
    let mut destination_connection = Connection::open(destination).map_err(|error| {
        CoreError::new(
            codes::INTERNAL_ERROR,
            format!("cannot open backup destination: {error}"),
        )
    })?;
    let backup = rusqlite::backup::Backup::new(&source_connection, &mut destination_connection)
        .map_err(|error| {
            CoreError::new(
                codes::INTERNAL_ERROR,
                format!("cannot create backup: {error}"),
            )
        })?;
    backup
        .run_to_completion(64, std::time::Duration::from_millis(5), None)
        .map_err(|error| {
            CoreError::new(codes::INTERNAL_ERROR, format!("backup failed: {error}"))
        })?;
    Ok(())
}
