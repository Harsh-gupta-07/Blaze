#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: Option<u64>,
    pub modified: Option<i64>,
    pub kind: String,
    pub indexed: i32,
}