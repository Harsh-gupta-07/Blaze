use rusqlite::{params, Connection, Result};

use crate::types::FileEntry;

pub fn get_connection() -> Result<Connection> {
    let conn = Connection::open("./db/main.db")?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        PRAGMA mmap_size = 30000000000;
        "
    )?;

    Ok(conn)
}

pub fn initialize_db() -> Result<()> {
    let conn = Connection::open("./db/main.db")?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS directories (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS files (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            directory_id INTEGER NOT NULL,
            name         TEXT NOT NULL,
            size         INTEGER,
            modified     INTEGER,
            kind         TEXT,
            indexed      INTEGER DEFAULT 0,

            FOREIGN KEY(directory_id)
            REFERENCES directories(id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_file
        ON files(directory_id, name);

        CREATE INDEX IF NOT EXISTS idx_name
        ON files(name);

        CREATE INDEX IF NOT EXISTS idx_modified
        ON files(modified);

        CREATE INDEX IF NOT EXISTS idx_directory_id
        ON files(directory_id);
        ",
    )?;

    Ok(())
}

const BATCH_SIZE: usize = 1000;

pub fn add_files(
    files: &[FileEntry],
    conn: &mut Connection,
) -> Result<()> {
    for chunk in files.chunks(BATCH_SIZE) {
        let tx = conn.transaction()?;

        {
            let mut dir_stmt = tx.prepare(
                "
                INSERT OR IGNORE INTO directories (path)
                VALUES (?1)
                ",
            )?;

            let mut dir_id_stmt = tx.prepare(
                "
                SELECT id
                FROM directories
                WHERE path = ?1
                ",
            )?;

            let mut file_stmt = tx.prepare(
                "
                INSERT OR REPLACE INTO files (
                    directory_id,
                    name,
                    size,
                    modified,
                    kind,
                    indexed
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
            )?;

            for file in chunk {
                // Insert directory once
                dir_stmt.execute(params![&file.parent])?;

                // Fetch directory id
                let directory_id: i64 = dir_id_stmt.query_row(
                    params![&file.parent],
                    |row| row.get(0),
                )?;

                // Insert file
                file_stmt.execute(params![
                    directory_id,
                    &file.name,
                    file.size.map(|v| v as i64),
                    file.modified,
                    &file.kind,
                    file.indexed,
                ])?;
            }
        }

        tx.commit()?;
    }

    Ok(())
}

pub fn get_files(
    conn: &Connection,
) -> Result<Vec<FileEntry>> {
    let mut stmt = conn.prepare(
        "
        SELECT
            files.id,
            directories.path,
            files.name,
            files.size,
            files.modified,
            files.kind,
            files.indexed
        FROM files
        JOIN directories
            ON files.directory_id = directories.id
        ORDER BY files.id DESC
        limit 100
        ",
    )?;

    let file_iter = stmt.query_map([], |row| {
        let parent: String = row.get(1)?;
        let name: String = row.get(2)?;

        Ok(FileEntry {
            id: row.get(0)?,

            path: format!("{}/{}", parent, name),

            parent,

            name,

            size: row
                .get::<_, Option<i64>>(3)?
                .map(|v| v as u64),

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