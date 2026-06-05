use fsevent_stream::stream::Event;

use tokio::sync::mpsc::Receiver;

use blaze_core::{db, tantivy, walker};

use rusqlite::Connection;

use std::{
    collections::HashSet,
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
enum ActionHint {
    SyncPath,
    Remove,
    Ignore,
}

struct PendingPathEvent {
    action: ActionHint,
    label: String,
    path: PathBuf,
}

/// Run the live indexer loop.
///
/// Receives `FsEvent`s from the watcher via a tokio mpsc channel,
/// batches them with a 50 ms window, de-duplicates by path, and
/// applies each change to SQLite + Tantivy.
///
/// After each batch the latest FSEvents event-ID is persisted to the
/// DB `metadata` table so the watcher can resume from that point
/// after a restart.
///
/// This function is meant to be called from
/// `tokio::task::spawn_blocking` because it performs blocking I/O
/// (SQLite, Tantivy) and uses `blocking_recv` on the channel.
pub fn run_indexer(mut rx: Receiver<Event>) {
    println!("[indexer] starting");

    let conn = match db::get_connection() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!(
                "Failed to open index database: {}",
                err,
            );
            return;
        }
    };

    let mut tantivy =
        match tantivy::initialize_index() {
            Ok(tantivy) => tantivy,
            Err(err) => {
                eprintln!(
                    "Failed to initialize Tantivy: {}",
                    err,
                );
                return;
            }
        };

    let mut buffer: Vec<Event> = Vec::new();

    loop {
        // Block until the first event arrives.
        let first = match rx.blocking_recv() {
            Some(event) => event,
            None => {
                eprintln!(
                    "Indexer channel closed; shutting down.",
                );
                break;
            }
        };

        buffer.push(first);

        // Drain the channel for up to 50 ms to coalesce
        // rapid bursts into a single batch.
        let deadline =
            Instant::now()
                + Duration::from_millis(50);

        let mut disconnected = false;

        while Instant::now() < deadline {
            match rx.try_recv() {
                Ok(event) => {
                    buffer.push(event);
                }

                Err(
                    tokio::sync::mpsc::error::TryRecvError::Empty,
                ) => {
                    std::thread::sleep(
                        Duration::from_millis(5),
                    );
                }

                Err(
                    tokio::sync::mpsc::error::TryRecvError::Disconnected,
                ) => {
                    disconnected = true;
                    break;
                }
            }
        }

        let pending_events =
            collect_pending_events(
                &mut buffer,
            );

        // Track the highest event ID in this batch so
        // we can persist it as the resume checkpoint.
        let max_event_id: u64 = pending_events
            .iter()
            .map(|e| e.event_id)
            .max()
            .unwrap_or(0);

        println!(
            "[indexer] processing batch of {} unique paths (max event ID: {})",
            pending_events.len(),
            max_event_id,
        );

        for event in pending_events {
            if let Err(err) = process_event(
                &conn,
                &mut tantivy,
                event.inner,
            ) {
                eprintln!(
                    "Failed to process fs event: {}",
                    err,
                );
            }
        }

        if let Err(err) =
            tantivy::commit(&mut tantivy)
        {
            eprintln!(
                "Failed to commit Tantivy batch: {}",
                err,
            );
        }

        // Persist the checkpoint so the watcher can
        // resume from this event ID after a restart.
        if max_event_id > 0 {
            if let Err(err) = db::set_metadata(
                &conn,
                "last_fsevent_id",
                &max_event_id.to_string(),
            ) {
                eprintln!(
                    "Failed to persist event ID checkpoint: {}",
                    err,
                );
            }
        }

        if disconnected {
            eprintln!(
                "Indexer channel disconnected; shutting down.",
            );
            break;
        }
    }
}

fn process_event(
    conn: &Connection,
    tantivy: &mut tantivy::TantivyState,
    event: PendingPathEvent,
) -> Result<(), String> {
    let full_path = event.path.to_string_lossy().to_string();

    match event.action {
        ActionHint::SyncPath => {
            let metadata =
                match std::fs::symlink_metadata(
                    &event.path,
                ) {
                    Ok(metadata) => metadata,
                    Err(err)
                        if err.kind()
                            == ErrorKind::NotFound =>
                    {
                        println!(
                            "[indexer] {} missing at process time; treating as delete {}",
                            event.label,
                            full_path,
                        );

                        delete_path(
                            conn,
                            tantivy,
                            &event.path,
                            &event.label,
                        )?;

                        return Ok(());
                    }

                    Err(err) => {
                        return Err(format!(
                            "Failed to stat {}: {}",
                            full_path,
                            err,
                        ));
                    }
                };

            let kind = walker::file_kind(
                &metadata.file_type(),
            );

            let file = walker::file_entry_from_path(
                &event.path,
                &metadata,
                kind,
                1,
            )
            .ok_or_else(|| {
                format!(
                    "Path has no terminal name: {}",
                    full_path,
                )
            })?;

            db::upsert_file(conn, &file)
                .map_err(|err| err.to_string())?;

            tantivy::update_document(
                tantivy,
                &file,
            )
            .map_err(|err| err.to_string())?;

            println!(
                "[indexer] {} upserted {} ({})",
                event.label,
                file.path,
                file.kind,
            );
        }

        ActionHint::Remove => {
            delete_path(
                conn,
                tantivy,
                &event.path,
                &event.label,
            )?;
        }

        ActionHint::Ignore => {
            println!(
                "[indexer] ignoring {} for {}",
                event.label,
                full_path,
            );
        }
    }

    Ok(())
}

struct PendingPathEventWithId {
    inner: PendingPathEvent,
    event_id: u64,
}

/// Flatten a buffer of raw `FsEvent`s into a deduplicated
/// list of actionable path events, keeping only the latest
/// action per path.
fn collect_pending_events(
    buffer: &mut Vec<Event>,
) -> Vec<PendingPathEventWithId> {
    let mut flattened: Vec<PendingPathEventWithId> = Vec::new();

    for event in buffer.drain(..) {
        let action =
            classify_fsevent_flags(event.raw_flags);
        let label =
            format!("flags={:x}", event.raw_flags);
        let path = event.path.clone();

        if walker::should_ignore_path(&path)
        {
            println!(
                "[indexer] filtered ignored path {}",
                path.to_string_lossy(),
            );
            continue;
        }

        flattened.push(PendingPathEventWithId {
            inner: PendingPathEvent {
                action,
                label,
                path,
            },
            event_id: event.id,
        });
    }

    let original_count = flattened.len();
    let mut seen = HashSet::new();
    let mut deduped: Vec<PendingPathEventWithId> = Vec::new();

    // Walk backwards so we keep the *latest* action for each path.
    for event in flattened.into_iter().rev() {
        let key =
            event.inner.path.to_string_lossy().to_string();

        if seen.insert(key) {
            deduped.push(event);
        }
    }

    deduped.reverse();

    if original_count != deduped.len() {
        println!(
            "[indexer] deduped batch paths from {} to {}",
            original_count,
            deduped.len(),
        );
    }

    deduped
}

// Use raw flag bits directly for classification to avoid
// depending on the exact bitflags version re-exported by
// fsevent-stream.
const ITEM_REMOVED: u32 = 0x0000_0200;
const ITEM_CREATED: u32 = 0x0000_0100;
const ITEM_MODIFIED: u32 = 0x0000_1000;
const ITEM_RENAMED: u32 = 0x0000_0800;

fn classify_fsevent_flags(
    raw_flags: u32,
) -> ActionHint {
    if raw_flags & ITEM_REMOVED != 0 {
        ActionHint::Remove
    } else if raw_flags
        & (ITEM_CREATED | ITEM_MODIFIED | ITEM_RENAMED)
        != 0
    {
        ActionHint::SyncPath
    } else {
        ActionHint::Ignore
    }
}

fn delete_path(
    conn: &Connection,
    tantivy: &mut tantivy::TantivyState,
    path: &std::path::Path,
    reason: &str,
) -> Result<(), String> {
    let full_path = path.to_string_lossy().to_string();
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let is_dir =
        db::is_directory(conn, &parent, &name)
            .unwrap_or(false);

    if is_dir {
        let subtree_paths =
            db::get_subtree_paths(
                conn,
                &full_path,
            )
            .map_err(|err| err.to_string())?;

        db::delete_directory_recursive(
            conn,
            &full_path,
        )
        .map_err(|err| err.to_string())?;

        tantivy::delete_documents(
            tantivy,
            &subtree_paths,
        );

        println!(
            "[indexer] {} removed directory {} ({} descendants)",
            reason,
            full_path,
            subtree_paths.len(),
        );
    } else {
        db::delete_file(
            conn,
            &parent,
            &name,
        )
        .map_err(|err| err.to_string())?;

        tantivy::delete_document(
            tantivy,
            &full_path,
        );

        println!(
            "[indexer] {} removed file {}",
            reason,
            full_path,
        );
    }

    Ok(())
}
