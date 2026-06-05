use std::{process, sync::Arc, thread};

use blaze_core::{db, tantivy, walker};

use crate::{indexed, watcher, FsEvent};

/// The filesystem root Blaze indexes and watches.
pub const WATCH_ROOT: &str = ".";

/// Initialise the database, perform a warm or cold start,
/// then launch the FSEvents watcher + live indexer in the
/// background.
///
/// This function returns as soon as both background tasks
/// are running — the watcher and indexer run for the
/// lifetime of the process.
///
/// Safe to call from any thread; internally it creates a
/// dedicated tokio runtime so the caller does not need one.
pub fn start() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("blaze-daemon")
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    // Run the async startup on that runtime, but return
    // once the background tasks are spawned so the caller
    // (Tauri) can continue.
    rt.block_on(async {
        run_startup().await;
    });

    // Leak the runtime so its threads keep running after
    // this function returns.
    std::mem::forget(rt);
}

async fn run_startup() {
    println!("[daemon] initializing database");

    match db::initialize_db() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Failed to initialize database: {}", err);
            process::exit(1);
        }
    }

    // ---- Warm vs Cold start ----
    let conn = match db::get_connection() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to connect to DB: {}", err);
            process::exit(1);
        }
    };

    let since = match db::get_metadata(&conn, "last_fsevent_id") {
        Ok(Some(id_str)) => match id_str.parse::<u64>() {
            Ok(id) => {
                println!(
                    "[daemon] warm restart — resuming from FSEvents event ID {}",
                    id
                );
                id
            }
            Err(_) => {
                eprintln!(
                    "[daemon] invalid stored event ID '{}'; cold start",
                    id_str
                );
                cold_bootstrap();
                watcher::SINCE_NOW
            }
        },
        Ok(None) => {
            println!("[daemon] no stored event ID — cold start");
            cold_bootstrap();
            watcher::SINCE_NOW
        }
        Err(err) => {
            eprintln!("[daemon] failed to read metadata: {} — cold start", err);
            cold_bootstrap();
            watcher::SINCE_NOW
        }
    };

    drop(conn);

    // ---- Watcher + Indexer ----
    let (tx, rx) = tokio::sync::mpsc::channel::<FsEvent>(10_000);

    println!("[daemon] starting watcher on {}", WATCH_ROOT);

    tokio::spawn(async move {
        if let Err(err) = watcher::start_watcher(WATCH_ROOT, since, tx).await {
            eprintln!("Watcher failed: {}", err);
        }
    });

    println!("[daemon] live indexing active");

    // Indexer is blocking I/O — keep it off the async executor.
    tokio::task::spawn_blocking(move || {
        indexed::run_indexer(rx);
    });
    // Return immediately; the tasks run in background.
}

/// Full filesystem walk + index rebuild.
fn cold_bootstrap() {
    println!("[daemon] scanning {} for bootstrap index", WATCH_ROOT);
    let files = Arc::new(walker::scan_directory(WATCH_ROOT));
    println!("[daemon] bootstrap scan found {} entries", files.len());

    let db_files = Arc::clone(&files);
    let db_worker = thread::spawn(move || {
        let mut conn = db::get_connection()?;
        db::add_files(db_files.as_ref(), &mut conn)
    });

    let index_files = Arc::clone(&files);
    let mut tanti = match tantivy::initialize_index() {
        Ok(t) => t,
        Err(err) => {
            eprintln!("Unable to initialize Tantivy: {}", err);
            process::exit(1);
        }
    };
    let index_worker = thread::spawn(move || tantivy::make_index(index_files.as_ref(), &mut tanti));

    match db_worker.join() {
        Ok(Ok(_)) => println!("[daemon] bootstrap SQLite index complete"),
        Ok(Err(err)) => eprintln!("Failed to add files: {}", err),
        Err(_) => eprintln!("DB worker panicked"),
    }

    match index_worker.join() {
        Ok(Ok(_)) => println!("[daemon] bootstrap Tantivy index complete"),
        Ok(Err(err)) => eprintln!("Failed to create Tantivy index: {}", err),
        Err(_) => eprintln!("Index worker panicked"),
    }
}
