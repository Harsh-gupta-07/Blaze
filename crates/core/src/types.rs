use crate::defaults;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub id: i64,
    pub path: String,
    pub parent: String,
    pub name: String,
    pub size: Option<u64>,
    pub modified: Option<i64>,
    pub kind: String,
    pub indexed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    new_user: bool,
    theme: String,
    ignored_dirs: Vec<String>,
    walker_dirs: Vec<String>,
}

impl Default for UserData {
    fn default() -> Self {
        Self {
            new_user: true,
            theme: defaults::theme_default(),
            ignored_dirs: defaults::ignored_dirs_defaults(),
            walker_dirs: defaults::walker_dirs_defaults(),
        }
    }
}
