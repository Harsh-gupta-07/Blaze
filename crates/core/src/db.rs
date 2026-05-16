use rusqlite::{Connection, Result, params};

use crate::{types::FileEntry, walker};

pub fn get_connection() -> Result<Connection> {
    let conn = Connection::open("./.db/main.db")?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        PRAGMA mmap_size = 30000000000;
        ",
    )?;

    Ok(conn)
}

pub fn initialize_db() -> Result<()> {
    let conn = Connection::open("./.db/main.db")?;

    conn.execute_batch(
        "
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS directories (
                id   INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS files (
                id           INTEGER PRIMARY KEY,
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

    println!("[db] initialized schema");

    Ok(())
}

const BATCH_SIZE: usize = 1000;
pub fn add_files(files: &[FileEntry], conn: &mut Connection) -> Result<()> {
    println!(
        "[db] bulk indexing {} entries",
        files.len(),
    );

    for chunk in files.chunks(BATCH_SIZE) {
        let tx = conn.transaction()?;

        {
            let mut dir_stmt = tx.prepare(
                "
                INSERT INTO directories (id, path)
                VALUES (?1, ?2)
                ON CONFLICT(id) DO UPDATE SET
                    path = excluded.path
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
                INSERT INTO files (
                    id,
                    directory_id,
                    name,
                    size,
                    modified,
                    kind,
                    indexed
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)

                ON CONFLICT(directory_id, name)
                DO UPDATE SET
                    id = excluded.id,
                    size = excluded.size,
                    modified = excluded.modified,
                    kind = excluded.kind,
                    indexed = excluded.indexed
                ",
            )?;

            for file in chunk {
                let parent_id = walker::generate_id(&file.parent);

                // Insert directory once
                dir_stmt.execute(params![parent_id, &file.parent,])?;

                // Fetch directory id
                let directory_id: i64 =
                    dir_id_stmt.query_row(params![&file.parent], |row| row.get(0))?;

                // Insert file
                file_stmt.execute(params![
                    file.id,
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

        println!(
            "[db] committed bulk chunk of {} entries",
            chunk.len(),
        );
    }

    Ok(())
}

pub fn get_files(conn: &Connection) -> Result<Vec<FileEntry>> {
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

// pub fn get_connection(
// ) -> Result<Connection> {
//     let conn =
//         Connection::open("./db/main.db")?;

//     Ok(conn)
// }

pub fn upsert_file(conn: &Connection, file: &FileEntry) -> Result<()> {
    let parent_id = walker::generate_id(&file.parent);

    conn.execute(
        "
        INSERT INTO directories(id, path)
        VALUES(?1, ?2)
        ON CONFLICT(id) DO UPDATE SET
            path = excluded.path
        ",
        params![parent_id, &file.parent],
    )?;

    let directory_id: i64 = conn.query_row(
        "
        SELECT id
        FROM directories
        WHERE path = ?1
        ",
        [&file.parent],
        |row| row.get(0),
    )?;

    conn.execute(
        "
        INSERT INTO files (
            id,
            directory_id,
            name,
            size,
            modified,
            kind,
            indexed
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)

        ON CONFLICT(directory_id, name)
        DO UPDATE SET
            id = excluded.id,
            size = excluded.size,
            modified = excluded.modified,
            kind = excluded.kind,
            indexed = excluded.indexed
        ",
        params![
            file.id,
            directory_id,
            file.name,
            file.size.map(|v| v as i64),
            file.modified,
            file.kind,
            file.indexed,
        ],
    )?;

    println!(
        "[db] upserted {} ({})",
        file.path,
        file.kind,
    );

    Ok(())
}

pub fn delete_file(conn: &Connection, parent: &str, name: &str) -> Result<()> {
    conn.execute(
        "
        DELETE FROM files

        WHERE directory_id = (
            SELECT id
            FROM directories
            WHERE path = ?1
        )

        AND name = ?2
        ",
        params![parent, name,],
    )?;

    println!(
        "[db] deleted file {}/{}",
        parent,
        name,
    );

    Ok(())
}

pub fn get_subtree_paths(conn: &Connection, path: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "
        SELECT
            directories.path || '/' || files.name AS path
        FROM files
        JOIN directories
            ON files.directory_id = directories.id
        WHERE (directories.path || '/' || files.name) = ?1
        OR (directories.path || '/' || files.name) LIKE ?2
        ",
    )?;

    let path_iter = stmt.query_map(
        params![path, format!("{}/%", path)],
        |row| row.get(0),
    )?;

    let mut paths = Vec::new();

    for entry in path_iter {
        paths.push(entry?);
    }

    println!(
        "[db] loaded {} subtree paths for {}",
        paths.len(),
        path,
    );

    Ok(paths)
}

pub fn delete_directory_recursive(conn: &Connection, path: &str) -> Result<()> {
    conn.execute(
        "
        DELETE FROM files

        WHERE directory_id IN (
            SELECT id
            FROM directories
            WHERE path = ?1
            OR path LIKE ?2
        )
        ",
        params![path, format!("{}/%", path),],
    )?;

    conn.execute(
        "
        DELETE FROM directories

        WHERE path = ?1
        OR path LIKE ?2
        ",
        params![path, format!("{}/%", path),],
    )?;

    println!(
        "[db] deleted directory subtree {}",
        path,
    );

    Ok(())
}

pub fn is_directory(conn: &Connection, parent: &str, name: &str) -> Result<bool> {
    let result: String = conn.query_row(
        "
        SELECT kind
        FROM files

        WHERE directory_id = (
            SELECT id
            FROM directories
            WHERE path = ?1
        )

        AND name = ?2
        ",
        params![parent, name],
        |row| row.get(0),
    )?;

    Ok(result == "dir")
}
