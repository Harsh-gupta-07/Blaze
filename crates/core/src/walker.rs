use crate::types::FileEntry;
use jwalk::WalkDir;
use std::fs::{FileType, Metadata};
use std::collections::hash_map::DefaultHasher;
use std::hash::{
    Hash,
    Hasher,
};
use std::path::{
    Component,
    Path,
};
use std::time::UNIX_EPOCH;

// Ignore some dir to reduce point less overhead
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    ".parcel-cache",
    ".venv",
    "venv",
    "env",
    "coverage",
    ".gradle",
    ".terraform",
    ".db",
];


fn should_ignore_dir(name: &str) -> bool {
    IGNORED_DIRS.contains(&name)
}

pub fn should_ignore_path(
    path: &Path,
) -> bool {
    path.components().any(|component| {
        match component {
            Component::Normal(name) => {
                should_ignore_dir(
                    &name.to_string_lossy(),
                )
            }
            _ => false,
        }
    })
}

pub fn file_kind(
    file_type: &FileType,
) -> &'static str {
    if file_type.is_file() {
        "file"
    } else if file_type.is_dir() {
        "dir"
    } else if file_type.is_symlink() {
        "symlink"
    } else {
        "unknown"
    }
}

pub fn modified_timestamp(
    metadata: &Metadata,
) -> Option<i64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

pub fn file_entry_from_path(
    path: &Path,
    metadata: &Metadata,
    kind: &str,
    indexed: i32,
) -> Option<FileEntry> {
    let path_str = path.to_string_lossy().to_string();
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())?;

    Some(FileEntry {
        id: generate_id(&path_str),
        path: path_str,
        parent,
        name,
        size: Some(metadata.len()),
        modified: modified_timestamp(metadata),
        kind: kind.to_string(),
        indexed,
    })
}

pub fn scan_directory(root: &str) -> Vec<FileEntry> {
    WalkDir::new(root)
        .parallelism(jwalk::Parallelism::RayonNewPool(0)) 
        .skip_hidden(false)
        .process_read_dir(|_, _, _, children| {
            children.retain(|child| {
                let Ok(child) = child.as_ref() else {
                    return true;
                };

                let name = child.file_name.to_string_lossy();
                !child.file_type.is_dir() || !should_ignore_dir(name.as_ref())
            });
        })
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;

            let metadata = entry.metadata().ok()?;
            let file_type = entry.file_type();
            let kind = file_kind(&file_type);

            file_entry_from_path(
                &entry.path(),
                &metadata,
                kind,
                0,
            )
        })
        .collect()
}


pub fn generate_id(
    path: &str,
) -> i64 {
    let mut hasher =
        DefaultHasher::new();

    path.hash(&mut hasher);

    hasher.finish() as i64
}
