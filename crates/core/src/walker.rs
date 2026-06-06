use crate::{db::app_data_dir, types::FileEntry};
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
    "__pycache__",
    "node_modules",
];


/// Returns `true` if a directory (or any path component) named `name`
/// should be excluded from the index.
///
/// Excluded names:
/// - Any name starting with `.` (hidden files/dirs on Unix/macOS).
/// - Names in the `IGNORED_DIRS` allowlist (build artefacts, caches, …).
fn should_ignore_dir(name: &str) -> bool {
    name.starts_with('.') || IGNORED_DIRS.contains(&name)
}

/// Returns `true` if `path` should be excluded from the index.
///
/// Three conditions trigger exclusion:
/// 1. Any path component matches `should_ignore_dir` (hidden names, build
///    artefacts, caches, etc.).
/// 2. The terminal filename itself is hidden (starts with `.`). This covers
///    plain hidden files like `.DS_Store` or `.env` that sit inside an
///    otherwise-visible directory.
/// 3. The path is inside the Blaze Application Support directory
///    (`~/Library/Application Support/com.Harsh.Blaze`), which contains
///    the SQLite DB and Tantivy index — we must never re-index our own
///    data store.
pub fn should_ignore_path(
    path: &Path,
) -> bool {
    // Check 1: ignored/hidden component anywhere in the path.
    if path.components().any(|component| {
        match component {
            Component::Normal(name) => {
                should_ignore_dir(
                    &name.to_string_lossy(),
                )
            }
            _ => false,
        }
    }) {
        return true;
    }

    // Check 2: Blaze data directory prefix.
    let data_dir = app_data_dir();
    if path.starts_with(&data_dir) {
        return true;
    }

    false
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
        .skip_hidden(true)  // skip hidden files and dirs (names starting with '.')
        .process_read_dir(|_, path, _, children| {
            // Also prune non-hidden ignored directories (build artefacts, caches…)
            // so jwalk never descends into them.
            let data_dir = app_data_dir();
            children.retain(|child| {
                let Ok(child) = child.as_ref() else {
                    return true;
                };

                // Always prune the Blaze data directory.
                if child.path().starts_with(&data_dir) {
                    return false;
                }

                let name = child.file_name.to_string_lossy();
                // skip_hidden already handles dot-names; keep this guard
                // for the non-hidden IGNORED_DIRS entries.
                !child.file_type.is_dir() || !IGNORED_DIRS.contains(&name.as_ref())
            });
            let _ = path; // suppress unused-variable warning
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
