use rusqlite::{params,Connection, Result};
use crate::types::FileEntry;

pub fn get_connection() -> Result<Connection> {
    Connection::open("./db/main.db")
}

pub fn initialize_db() -> Result<()> {
    let conn = Connection::open("./db/main.db")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
        id       INTEGER PRIMARY KEY AUTOINCREMENT,
        path     TEXT NOT NULL UNIQUE,
        name     TEXT NOT NULL,
        size     INTEGER,
        modified INTEGER,  
        kind     TEXT,      
        indexed  INTEGER DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_name
        ON files(name);

        CREATE INDEX IF NOT EXISTS idx_modified
        ON files(modified);",
    )?;
    Ok(())
}



const BATCH_SIZE: usize = 1000;
pub fn add_files(f: Vec<FileEntry>, conn: &mut Connection)->Result<()>{
    for chunk in f.chunks(BATCH_SIZE){
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "
                INSERT OR REPLACE INTO FILES (
                    path,
                    name,
                    size,
                    modified,
                    kind,
                    indexed
                )
                VALUES (?1,?2,?3,?4,?5,?6)
                ",
            )?;

            for file in chunk{
                stmt.execute(params![
                    file.path,
                    file.name,
                    file.size.map(|v| v as i64),
                    file.modified,
                    file.kind, 
                    file.indexed
                ])?;
            }
        }
        tx.commit()?;
    }

    Ok(())
}


pub fn get_files(conn: &Connection) -> Result<Vec<FileEntry>> {
    let mut stmt = conn.prepare(
        "
        SELECT
            id,
            path,
            name,
            size,
            modified,
            kind,
            indexed
        FROM files order by id desc Limit 100
        "
    )?;

    let file_iter = stmt.query_map([], |row| {
        Ok(FileEntry {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            size: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
            modified: row.get(4)?,
            kind: row.get(5)?,
            indexed: row.get(6)?,
        })
    })?;

    let mut files = Vec::new();

    for file in file_iter {
        files.push(file?);
    }

    Ok(files)
}

