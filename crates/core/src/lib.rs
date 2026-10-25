//! Project Knowledge Core —— 索引与查询的唯一实现。
//!
//! 职责边界（见 `docs/NEW-PROJECT-STRUCTURE.md` 第 3 节）：
//! - 统一拥有扫描、运行提取器、校验、事务、索引、检索、追踪、增量与材料输出；
//! - 不负责模型调用：Agent 侧通过本 crate 的公开用例组合 Prompt 与工具；
//! - `contract` 是共用类型的唯一来源，JSON Schema 由它导出，Node 侧类型再由 Schema 生成；
//! - `storage` 是数据库写入的唯一入口，其他模块只能通过它使用事务与查询。

/// 代码地图：面向当前问题的相关符号与文件轮廓。
pub mod codemap;
/// 外部提取器宿主：manifest、注册、运行、校验与导入。
pub mod collect;
/// 共用数据与接口契约。
pub mod contract;
/// 证据读取与核验。
pub mod evidence;
/// 通用关系查询（上下游、有界路径）。
pub mod graph;
/// 索引构建与增量更新。
pub mod index;
/// 知识材料组织与输出。
pub mod knowledge;
/// 项目接入与来源管理。
pub mod project;
/// 需求定位与三个业务场景的事实组合。
pub mod query;
/// 统一文本与事实搜索。
pub mod search;
/// SQLite 持久化与事务。
pub mod storage;
/// 可选通用语法能力（Tree-sitter）。
pub mod syntax;
