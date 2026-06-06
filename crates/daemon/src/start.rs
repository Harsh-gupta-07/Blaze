use std::sync::OnceLock;
use std::{process, sync::Arc, thread};

use tokio::sync::watch;

use blaze_core::{db::{self, app_data_dir}, tantivy, walker};

use crate::{indexed, watcher, FsEvent};

/// The filesystem root Blaze indexes and watches.
pub const WATCH_ROOT: &str = "/Users";

/// Global shutdown sender.  Calling `shutdown()` sets
/// this to `true`, which the watcher picks up via its
/// `watch::Receiver`.
static SHUTDOWN_TX: OnceLock<watch::Sender<bool>> = OnceLock::new();

/// Trigger a graceful shutdown of the daemon.
///
/// - The watcher sees the signal and stops, dropping
///   its channel sender.
/// - The indexer drains any remaining events, commits
///   Tantivy, persists the last event ID, then exits.
///
/// Safe to call from any thread, any number of times.
pub fn shutdown() {
    if let Some(tx) = SHUTDOWN_TX.get() {
        let _ = tx.send(true);
        println!("[daemon] shutdown signal sent");
    }
}

/// Initialise the database, perform a warm or cold start,
/// then launch the FSEvents watcher + live indexer.
///
/// **This function blocks** until a shutdown signal is
/// received (SIGTERM, SIGINT, or a call to `shutdown()`).
/// When running inside Tauri, call this from a background
/// thread so the UI remains responsive.
pub fn start() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("blaze-daemon")
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    rt.block_on(async {
        run_startup().await;
    });

    // The runtime drops here — all tasks are already
    // finished so this is a clean teardown.
    println!("[daemon] runtime shut down cleanly");
}

async fn run_startup() {
    println!("[daemon] initializing database");

    // ---- App-Support directory check ----
    // Determine whether the data directories exist *before* we create them.
    // If they are absent this is guaranteed to be a fresh install / first run,
    // so we must do a cold start even if a DB file is later seeded somehow.
    let data_dir   = app_data_dir();
    let db_dir     = data_dir.join("db");
    let tantivy_dir = data_dir.join("db/tantivy");

    let dirs_existed = db_dir.exists() && tantivy_dir.exists();

    if !dirs_existed {
        println!(
            "[daemon] data directories not found at {} — creating and forcing cold start",
            data_dir.display()
        );

        if let Err(err) = std::fs::create_dir_all(&tantivy_dir) {
            eprintln!("[daemon] failed to create data directories: {}", err);
            process::exit(1);
        }
    }

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

    let since = if !dirs_existed {
        // Data directories were just created — always cold start.
        println!("[daemon] fresh data directory — cold start");
        cold_bootstrap();
        watcher::SINCE_NOW
    } else {
        match db::get_metadata(&conn, "last_fsevent_id") {
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
        }
    };

    drop(conn);

    // ---- Shutdown channel ----
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    SHUTDOWN_TX.set(shutdown_tx).ok();

    // ---- Watcher + Indexer ----
    let (tx, rx) = tokio::sync::mpsc::channel::<FsEvent>(10_000);

    println!("[daemon] starting watcher on {}", WATCH_ROOT);

    tokio::spawn(async move {
        if let Err(err) =
            watcher::start_watcher(WATCH_ROOT, since, tx, shutdown_rx).await
        {
            eprintln!("Watcher failed: {}", err);
        }
    });

    println!("[daemon] live indexing active");

    // ---- Signal listener ----
    tokio::spawn(async {
        wait_for_signal().await;
        shutdown();
    });

    // ---- Indexer (blocking) ----
    // We await the indexer handle so that `run_startup`
    // (and therefore `start()`) blocks until the indexer
    // finishes draining after a shutdown signal.
    let indexer_handle = tokio::task::spawn_blocking(move || {
        indexed::run_indexer(rx);
    });

    match indexer_handle.await {
        Ok(_) => println!("[daemon] indexer shut down cleanly"),
        Err(err) => eprintln!("[daemon] indexer task panicked: {}", err),
    }
}

/// Wait for SIGTERM or SIGINT (Ctrl-C).
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm =
        signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
    let mut sigint =
        signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            println!("[daemon] received SIGTERM");
        }
        _ = sigint.recv() => {
            println!("[daemon] received SIGINT (Ctrl-C)");
        }
    }
}

/// Full filesystem walk + index rebuild, with a
/// generation-based sweep to purge files that were
/// deleted while Blaze was offline.
fn cold_bootstrap() {
    // Pick a generation stamp for this boot.
    let generation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System clock before UNIX epoch")
        .as_secs() as i64;

    println!(
        "[daemon] scanning {} for bootstrap index (generation {})",
        WATCH_ROOT, generation,
    );

    let files = Arc::new(walker::scan_directory(WATCH_ROOT));
    println!("[daemon] bootstrap scan found {} entries", files.len());

    // ---- Parallel DB + Tantivy upsert ----
    let db_files = Arc::clone(&files);
    let db_worker = thread::spawn(move || {
        let mut conn = db::get_connection()?;
        db::add_files(db_files.as_ref(), &mut conn, generation)
    });

    let index_files = Arc::clone(&files);
    let mut tanti = match tantivy::initialize_index() {
        Ok(t) => t,
        Err(err) => {
            eprintln!("Unable to initialize Tantivy: {}", err);
            process::exit(1);
        }
    };
    let index_worker = thread::spawn(move || {
        tantivy::make_index(index_files.as_ref(), &mut tanti)
    });

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

    // ---- Sweep stale rows ----
    let conn = match db::get_connection() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("[daemon] sweep: failed to connect to DB: {}", err);
            return;
        }
    };

    let stale_paths = match db::get_stale_paths(&conn, generation) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[daemon] sweep: failed to query stale paths: {}", err);
            return;
        }
    };

    if stale_paths.is_empty() {
        println!("[daemon] sweep: no stale files to remove");
        return;
    }

    println!(
        "[daemon] sweep: found {} stale files to purge",
        stale_paths.len(),
    );

    if let Err(err) = db::delete_stale_files(&conn, generation) {
        eprintln!("[daemon] sweep: failed to delete stale rows: {}", err);
        return;
    }

    match tantivy::initialize_index() {
        Ok(mut tanti) => {
            tantivy::delete_documents(&mut tanti, &stale_paths);
            if let Err(err) = tantivy::commit(&mut tanti) {
                eprintln!("[daemon] sweep: Tantivy commit failed: {}", err);
            } else {
                println!(
                    "[daemon] sweep: purged {} deleted files from Tantivy",
                    stale_paths.len(),
                );
            }
        }
        Err(err) => {
            eprintln!("[daemon] sweep: failed to open Tantivy: {}", err);
        }
    }
}
