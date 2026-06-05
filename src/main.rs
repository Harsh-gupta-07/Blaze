use blaze_core::{db, tantivy, walker};
use blaze_daemon::{indexed, watcher, FsEvent};
use std::{process, sync::Arc, thread};

const WATCH_ROOT: &str = "/Users";

#[tokio::main]
async fn main() {
    println!(
        "[main] initializing database",
    );

    match db::initialize_db() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Failed to initialize database: {}", err);
            process::exit(1);
        }
    }

    // ----- Warm vs Cold start -----
    // Try to load the last persisted FSEvents event ID.
    // If found, FSEvents will replay every event since
    // that ID (journal replay), catching anything missed
    // while Blaze was down.
    let conn = match db::get_connection() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to connect to DB: {}", err);
            process::exit(1);
        }
    };

    let since = match db::get_metadata(&conn, "last_fsevent_id") {
        Ok(Some(id_str)) => {
            match id_str.parse::<u64>() {
                Ok(id) => {
                    println!(
                        "[main] warm restart — resuming from FSEvents event ID {}",
                        id,
                    );
                    id
                }
                Err(_) => {
                    eprintln!(
                        "[main] invalid stored event ID '{}'; falling back to cold start",
                        id_str,
                    );
                    cold_bootstrap();
                    watcher::SINCE_NOW
                }
            }
        }
        Ok(None) => {
            println!("[main] no stored event ID — cold start");
            cold_bootstrap();
            watcher::SINCE_NOW
        }
        Err(err) => {
            eprintln!(
                "[main] failed to read metadata: {} — cold start",
                err,
            );
            cold_bootstrap();
            watcher::SINCE_NOW
        }
    };

    drop(conn);

    // ----- Live watcher + indexer -----
    let (tx, rx) =
        tokio::sync::mpsc::channel::<FsEvent>(10_000);

    println!(
        "[main] starting watcher and live indexer on {}",
        WATCH_ROOT,
    );

    // Watcher: async task driving the FSEvents stream.
    tokio::spawn(async move {
        if let Err(err) =
            watcher::start_watcher(
                WATCH_ROOT,
                since,
                tx,
            )
            .await
        {
            eprintln!(
                "Watcher failed: {}",
                err,
            );
        }
    });

    println!(
        "[main] live indexing active; modify files under {} to test",
        WATCH_ROOT,
    );

    // Indexer: blocking task (SQLite + Tantivy I/O).
    tokio::task::spawn_blocking(move || {
        indexed::run_indexer(rx);
    })
    .await
    .unwrap_or_else(|err| {
        eprintln!("Indexer task panicked: {}", err);
    });
}

/// Full bootstrap: walk the entire filesystem, populate
/// SQLite and Tantivy from scratch.
fn cold_bootstrap() {
    println!(
        "[main] scanning {} for bootstrap index",
        WATCH_ROOT,
    );
    let files = Arc::new(walker::scan_directory(WATCH_ROOT));
    println!(
        "[main] bootstrap scan found {} entries",
        files.len(),
    );

    let db_files = Arc::clone(&files);
    let db_worker = thread::spawn(move || {
        let mut conn = db::get_connection()?;
        db::add_files(db_files.as_ref(), &mut conn)
    });

    let index_files = Arc::clone(&files);
    let mut tanti = match tantivy::initialize_index() {
        Ok(tanti) => tanti,
        Err(err) => {
            eprintln!("Unable to initialize Tantivy db {}", err);
            process::exit(1)
        }
    };
    let index_worker = thread::spawn(move || {
        tantivy::make_index(
            index_files.as_ref(),
            &mut tanti,
        )
    });

    match db_worker.join() {
        Ok(Ok(_)) => {
            println!(
                "[main] bootstrap SQLite index complete",
            );
        }
        Ok(Err(err)) => {
            eprintln!("Failed to Add Files: {}", err);
        }
        Err(_) => {
            eprintln!("DB worker panicked");
        }
    }

    match index_worker.join() {
        Ok(Ok(_)) => {
            println!(
                "[main] bootstrap Tantivy index complete",
            );
        }
        Ok(Err(err)) => {
            eprintln!("Failed to Create tantivy index {}", err);
        }
        Err(_) => {
            eprintln!("Index worker panicked");
        }
    }
}

#[allow(dead_code)]
fn print_sample_rows() {
    let conn = match db::get_connection() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to connect to DB: {}", err);
            return;
        }
    };

    let fetch = match db::get_files(&conn) {
        Ok(fetch) => fetch,
        Err(err) => {
            eprintln!("Error Fetching files List: {}", err);
            return;
        }
    };

    println!("{:#?}", fetch)
}
