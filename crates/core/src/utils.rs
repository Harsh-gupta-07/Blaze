use std::{env, path::PathBuf};

/// Returns the base application-data directory:
///   `~/Library/Application Support/com.Blaze.Harsh`
///
/// Override by setting the `BLAZE_DATA_DIR` environment variable.
/// This is also used by `tantivy.rs` for the Tantivy index path.
pub fn app_data_dir() -> PathBuf {
    if let Ok(path) = env::var("BLAZE_DATA_DIR") {
        return PathBuf::from(path);
    }

    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Library/Application Support")
        })
        .join("com.Harsh.Blaze")
}