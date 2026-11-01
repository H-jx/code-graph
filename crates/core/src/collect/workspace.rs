//! 暂存目录的进程所有权；恢复只清理同项目、带本模块标记且已释放 OS 锁的目录。
use crate::contract::error::{CoreError, codes};
use crate::project::config::LoadedProject;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::new(codes::INTERNAL_ERROR, format!("staging workspace: {error}"))
}

pub(crate) struct StagingWorkspace {
    // 字段按声明顺序析构：Windows 上必须先关闭锁文件再删除 TempDir。
    _owner: File,
    directory: tempfile::TempDir,
}
impl Drop for StagingWorkspace {
    fn drop(&mut self) {
        let _ = self._owner.unlock();
    }
}
impl StagingWorkspace {
    pub fn new(project: &LoadedProject) -> Result<Self, CoreError> {
        let directory = tempfile::Builder::new()
            .prefix("stage_")
            .tempdir_in(root(project)?)
            .map_err(io_error)?;
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(directory.path().join("owner.lock"))
            .map_err(io_error)?;
        owner
            .try_lock()
            .map_err(|_| CoreError::new(codes::BUSY, "cannot own staging workspace"))?;
        std::fs::write(
            directory.path().join("owner.marker"),
            directory
                .path()
                .file_name()
                .expect("staging directory")
                .to_string_lossy()
                .as_bytes(),
        )
        .map_err(io_error)?;
        Ok(Self {
            _owner: owner,
            directory,
        })
    }
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
}

fn root(project: &LoadedProject) -> Result<PathBuf, CoreError> {
    let base = std::fs::canonicalize(&project.base_dir).map_err(io_error)?;
    let root = project.base_dir.join(".project-knowledge/work/staging");
    std::fs::create_dir_all(&root).map_err(io_error)?;
    let root = std::fs::canonicalize(root).map_err(io_error)?;
    if !root.starts_with(&base) {
        return Err(CoreError::new(
            codes::CONFIG_INVALID,
            "staging root resolves outside project workspace",
        ));
    }
    Ok(root)
}

pub(crate) fn cleanup(project: &LoadedProject) -> Result<usize, CoreError> {
    let root = root(project)?;
    let mut removed = 0;
    for entry in std::fs::read_dir(&root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("stage_")
            || !entry
                .file_type()
                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
        {
            continue;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if std::fs::symlink_metadata(entry.path())
                .map_err(io_error)?
                .file_attributes()
                & 0x400
                != 0
            {
                continue;
            }
        }
        let marker = entry.path().join("owner.marker");
        if !std::fs::symlink_metadata(&marker)
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() < 128)
            || std::fs::read_to_string(&marker).ok().as_deref() != Some(name.as_str())
        {
            continue;
        }
        let Ok(owner) = OpenOptions::new()
            .read(true)
            .write(true)
            .open(entry.path().join("owner.lock"))
        else {
            continue;
        };
        if owner.try_lock().is_err() {
            continue;
        }
        // Windows 递归删除前复核最终绝对目标；目录只能是本项目 staging 根的直接子目录。
        let resolved = std::fs::canonicalize(entry.path()).map_err(io_error)?;
        if resolved.parent() != Some(root.as_path()) || !resolved.starts_with(&root) {
            continue;
        }
        drop(owner);
        std::fs::remove_dir_all(&resolved).map_err(io_error)?;
        removed += 1;
    }
    Ok(removed)
}
