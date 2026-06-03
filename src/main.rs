use blaze_core::{db, tantivy, walker};
use blaze_daemon::{indexed, watcher};
use crossbeam_channel::bounded;

use std::{process, sync::Arc, thread};

const WATCH_ROOT: &str = ".";

fn main() {
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

    let (tx, rx) = bounded(10_000);
    println!(
        "[main] starting watcher and live indexer on {}",
        WATCH_ROOT,
    );

    thread::spawn(move || {
        if let Err(err) =
            watcher::start_watcher(
                WATCH_ROOT,
                tx,
            )
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
    indexed::run_indexer(rx);
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
