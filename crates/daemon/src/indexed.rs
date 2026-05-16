use crossbeam_channel::{
    Receiver,
    RecvTimeoutError,
};

use notify::{Event, EventKind};

use blazefind_core::{db, tantivy, walker};

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

pub fn run_indexer(rx: Receiver<Event>) {
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

    let mut buffer = Vec::new();

    loop {
        let first = match rx.recv() {
            Ok(event) => event,

            Err(err) => {
                eprintln!(
                    "Indexer channel closed: {}",
                    err,
                );
                break;
            }
        };

        buffer.push(first);

        let deadline =
            Instant::now()
                + Duration::from_millis(50);

        let mut disconnected = false;

        while let Some(remaining) = deadline
            .checked_duration_since(Instant::now())
        {
            match rx.recv_timeout(remaining) {
                Ok(event) => {
                    buffer.push(event);
                }

                Err(
                    RecvTimeoutError::Timeout,
                ) => break,

                Err(
                    RecvTimeoutError::Disconnected,
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

        println!(
            "[indexer] processing batch of {} events ({} unique paths)",
            pending_events.len(),
            pending_events.len(),
        );

        for event in pending_events {
            if let Err(err) = process_event(
                &conn,
                &mut tantivy,
                event,
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

fn collect_pending_events(
    buffer: &mut Vec<Event>,
) -> Vec<PendingPathEvent> {
    let mut flattened = Vec::new();

    for event in buffer.drain(..) {
        let action =
            classify_event_kind(&event.kind);
        let label =
            format!("{:?}", event.kind);

        for path in event.paths {
            flattened.push(PendingPathEvent {
                action,
                label: label.clone(),
                path,
            });
        }
    }

    let original_count = flattened.len();
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for event in flattened.into_iter().rev() {
        let key =
            event.path.to_string_lossy().to_string();

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

fn classify_event_kind(
    kind: &EventKind,
) -> ActionHint {
    match kind {
        EventKind::Create(_)
        | EventKind::Modify(_) => {
            ActionHint::SyncPath
        }
        EventKind::Remove(_) => {
            ActionHint::Remove
        }
        _ => ActionHint::Ignore,
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
