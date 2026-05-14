use jwalk::WalkDir;
use std::time::UNIX_EPOCH;
use crate::types::FileEntry;

pub fn scan_directory(root: &str) -> Vec<FileEntry> {
    WalkDir::new(root)
        .parallelism(jwalk::Parallelism::RayonNewPool(0))
        .skip_hidden(false)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;

            let metadata = entry.metadata().ok();

            let size = metadata.as_ref().map(|m| m.len());

            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            let file_type = entry.file_type();

            let kind = if file_type.is_file() {
                "file"
            } else if file_type.is_dir() {
                "dir"
            } else if file_type.is_symlink() {
                "symlink"
            } else {
                "unknown"
            };

            Some(FileEntry {
                path: entry.path().to_string_lossy().to_string(),
                name: entry.file_name.to_string_lossy().to_string(),
                size,
                modified,
                kind: kind.to_string(),
                indexed: 0,
            })
        })
        .collect()
}

