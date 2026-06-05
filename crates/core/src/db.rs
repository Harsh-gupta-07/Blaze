use rusqlite::{Connection, Result, params};

use crate::{types::FileEntry, walker};
use std::{env, fs, path::PathBuf};

fn join_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else if parent == "/" {
        format!("/{}", name)
    } else {
        format!("{}/{}", parent, name)
    }
}

fn db_path() -> PathBuf {
    if let Ok(path) = env::var("BLAZE_DB_PATH") {
        return PathBuf::from(path);
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidates = [cwd.join(".db/main.db"), cwd.join("../.db/main.db")];

    for candidate in candidates {
        if candidate.parent().is_some_and(|parent| parent.exists()) {
            return candidate;
        }
    }

    cwd.join(".db/main.db")
}

pub fn get_connection() -> Result<Connection> {
    let path = db_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(path)?;

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
    let path = db_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(path)?;

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
                generation   INTEGER NOT NULL DEFAULT 0,

                FOREIGN KEY(directory_id)
                REFERENCES directories(id)
            );

            CREATE TABLE IF NOT EXISTS metadata (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
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
pub fn add_files(files: &[FileEntry], conn: &mut Connection, generation: i64) -> Result<()> {
    println!("[db] bulk indexing {} entries (generation {})", files.len(), generation);

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
                    indexed,
                    generation
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)

                ON CONFLICT(directory_id, name)
                DO UPDATE SET
                    id = excluded.id,
                    size = excluded.size,
                    modified = excluded.modified,
                    kind = excluded.kind,
                    indexed = excluded.indexed,
                    generation = excluded.generation
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
                    generation,
                ])?;
            }
        }

        tx.commit()?;

        println!("[db] committed bulk chunk of {} entries", chunk.len(),);
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
        LIMIT 100
        ",
    )?;

    let file_iter = stmt.query_map([], |row| {
        let parent: String = row.get(1)?;
        let name: String = row.get(2)?;

        Ok(FileEntry {
            id: row.get(0)?,

            path: join_path(&parent, &name),

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

pub fn get_dir_files(conn: &Connection, path: String) -> Result<Vec<FileEntry>> {
    let path = {
        let trimmed = path.trim_end_matches(['/', '\\']);

        if trimmed.is_empty() {
            "/".to_string()
        } else {
            trimmed.to_string()
        }
    };

    let mut entries = Vec::new();

    let mut file_stmt = conn.prepare(
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
        WHERE directories.path = ?1
        ORDER BY files.id DESC
        ",
    )?;

    let file_iter = file_stmt.query_map(params![&path], |row| {
        let parent: String = row.get(1)?;
        let name: String = row.get(2)?;

        Ok(FileEntry {
            id: row.get(0)?,
            path: join_path(&parent, &name),
            parent,
            name,
            size: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
            modified: row.get(4)?,
            kind: row.get(5)?,
            indexed: row.get(6)?,
        })
    })?;

    for file in file_iter {
        entries.push(file?);
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.kind.cmp(&b.kind)));

    // eprintln!("{:?}", entries);
    Ok(entries)
}

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
            indexed,
            generation
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s','now'))

        ON CONFLICT(directory_id, name)
        DO UPDATE SET
            id = excluded.id,
            size = excluded.size,
            modified = excluded.modified,
            kind = excluded.kind,
            indexed = excluded.indexed,
            generation = excluded.generation
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

    println!("[db] upserted {} ({})", file.path, file.kind,);

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

    println!("[db] deleted file {}/{}", parent, name,);

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

    let path_iter = stmt.query_map(params![path, format!("{}/%", path)], |row| row.get(0))?;

    let mut paths = Vec::new();

    for entry in path_iter {
        paths.push(entry?);
    }

    println!("[db] loaded {} subtree paths for {}", paths.len(), path,);

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

    println!("[db] deleted directory subtree {}", path,);

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

pub fn get_metadata(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );

    match result {
        Ok(val) => Ok(Some(val)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn set_metadata(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata(key, value) VALUES(?1, ?2)",
        params![key, value],
    )?;

    println!("[db] set metadata {} = {}", key, value);

    Ok(())
}

/// Return full paths of every file whose generation is
/// older than `current_gen` — i.e. files that were NOT
/// seen during the latest cold-bootstrap scan and are
/// therefore assumed deleted.
pub fn get_stale_paths(
    conn: &Connection,
    current_gen: i64,
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "
        SELECT
            directories.path || '/' || files.name AS full_path
        FROM files
        JOIN directories
            ON files.directory_id = directories.id
        WHERE files.generation < ?1
        ",
    )?;

    let rows = stmt.query_map(params![current_gen], |row| {
        row.get::<_, String>(0)
    })?;

    let mut paths = Vec::new();
    for row in rows {
        paths.push(row?);
    }

    Ok(paths)
}

/// Delete every file row whose generation is older than
/// `current_gen`, then clean up any directories that no
/// longer contain files.  Returns the number of file
/// rows removed.
pub fn delete_stale_files(
    conn: &Connection,
    current_gen: i64,
) -> Result<usize> {
    let removed = conn.execute(
        "DELETE FROM files WHERE generation < ?1",
        params![current_gen],
    )?;

    // Prune empty directories.
    conn.execute(
        "
        DELETE FROM directories
        WHERE id NOT IN (
            SELECT DISTINCT directory_id FROM files
        )
        ",
        [],
    )?;

    println!(
        "[db] swept {} stale file rows (generation < {})",
        removed, current_gen,
    );

    Ok(removed)
}
