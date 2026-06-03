use serde::Serialize;
#[derive(Debug, Clone,Serialize)]
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