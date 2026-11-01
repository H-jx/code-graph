//! Structured, bounded candidate bundles. No caller-supplied absolute write paths.
use crate::{
    contract::error::{CoreError, codes},
    project::config::LoadedProject,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateFile {
    pub path: String,
    pub content: String,
}

pub fn write_bundle(
    project: &LoadedProject,
    run_id: &str,
    revision: u32,
    files: &[CandidateFile],
) -> Result<PathBuf, CoreError> {
    let invalid = || {
        CoreError::new(
            codes::INVALID_REQUEST,
            "candidate bundle has an unsafe path or exceeds limits",
        )
    };
    if !run_id.starts_with("agent_")
        || !run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || files.is_empty()
        || files.len() > 64
        || files.iter().map(|f| f.content.len()).sum::<usize>() > 2_097_152
    {
        return Err(invalid());
    }
    let mut paths = BTreeSet::new();
    for file in files {
        if file.path.len() > 180
            || file.path.split('/').any(|part| {
                let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || part.ends_with(['.', ' '])
                    || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                        && stem.len() == 4
                        && stem.as_bytes()[3].is_ascii_digit())
                    || part.chars().any(|c| {
                        c.is_control()
                            || matches!(c, ':' | '\\' | '<' | '>' | '"' | '|' | '?' | '*')
                    })
            })
            || !paths.insert(file.path.to_lowercase())
        {
            return Err(invalid());
        }
    }
    let base = std::fs::canonicalize(&project.base_dir).map_err(|_| invalid())?;
    let mut root = base.clone();
    for part in ["work", "agent", run_id] {
        root.push(part);
        if !root.exists() {
            std::fs::create_dir(&root).map_err(|_| invalid())?;
        }
        if std::fs::symlink_metadata(&root)
            .map_err(|_| invalid())?
            .file_type()
            .is_symlink()
            || crate::project::scan::is_reparse_point(&root)
            || !std::fs::canonicalize(&root)
                .map_err(|_| invalid())?
                .starts_with(&base)
        {
            return Err(invalid());
        }
    }
    // A revision is a new directory. Failed/superseded candidates are never edited in place.
    let directory = root.join(format!("revision-{revision:03}"));
    std::fs::create_dir(&directory).map_err(|_| invalid())?;
    for file in files {
        let target = directory.join(&file.path);
        std::fs::create_dir_all(target.parent().expect("bundle parent")).map_err(|_| invalid())?;
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .and_then(|mut f| f.write_all(file.content.as_bytes()))
            .map_err(|_| invalid())?;
    }
    Ok(directory)
}

/// Read an owned draft for the next immutable revision, without following links.
pub fn read_bundle(
    project: &LoadedProject,
    run_id: &str,
    revision: u32,
) -> Result<Vec<CandidateFile>, CoreError> {
    let invalid = || CoreError::new(codes::INVALID_REQUEST, "invalid candidate draft workspace");
    if !run_id.starts_with("agent_")
        || !run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(invalid());
    }
    let base = std::fs::canonicalize(&project.base_dir).map_err(|_| invalid())?;
    let expected = base
        .join("work/agent")
        .join(run_id)
        .join(format!("revision-{revision:03}"));
    let root = std::fs::canonicalize(&expected).map_err(|_| invalid())?;
    if root != expected {
        return Err(invalid());
    }
    let mut directories = vec![root.clone()];
    let mut files = Vec::new();
    let mut total = 0;
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).map_err(|_| invalid())? {
            let entry = entry.map_err(|_| invalid())?;
            let meta = std::fs::symlink_metadata(entry.path()).map_err(|_| invalid())?;
            if meta.file_type().is_symlink()
                || crate::project::scan::is_reparse_point(&entry.path())
            {
                return Err(invalid());
            }
            if meta.is_dir() {
                directories.push(entry.path());
                continue;
            }
            total += meta.len();
            if total > 2_097_152 || files.len() >= 64 {
                return Err(invalid());
            }
            files.push(CandidateFile {
                path: entry
                    .path()
                    .strip_prefix(&root)
                    .map_err(|_| invalid())?
                    .to_string_lossy()
                    .replace('\\', "/"),
                content: std::fs::read_to_string(entry.path()).map_err(|_| invalid())?,
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
