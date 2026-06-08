use std::path::Path;
use std::time::Duration;

use fsevent_stream::ffi::{
    FSEventStreamEventId, kFSEventStreamCreateFlagFileEvents, kFSEventStreamCreateFlagNoDefer,
    kFSEventStreamCreateFlagUseCFTypes, kFSEventStreamCreateFlagUseExtendedData,
    kFSEventStreamEventIdSinceNow,
};
use fsevent_stream::stream::{Event, create_event_stream};
use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio::sync::watch;

use blaze_core::walker;

/// Sentinel value: start streaming from "now", ignoring
/// all historical events.
pub const SINCE_NOW: FSEventStreamEventId = kFSEventStreamEventIdSinceNow;

/// Start an FSEvents stream on `root` that replays every
/// event whose ID > `since`, then continues in real-time.
///
/// The stream runs until one of:
///   - `shutdown_rx` is signalled (graceful shutdown)
///   - the indexer's channel is full / closed
///   - the FSEvents stream ends (shouldn't happen)
///
/// Events for ignored paths (see `walker::should_ignore_path`)
/// are silently dropped before they reach the channel.
pub async fn start_watcher(
    root: &str,
    since: FSEventStreamEventId,
    tx: Sender<Event>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "[watcher] starting FSEvents watch on {} (since event ID: {})",
        root, since,
    );

    let (stream, _handler) = create_event_stream(
        [Path::new(root)],
        since,
        Duration::from_millis(200),
        kFSEventStreamCreateFlagNoDefer
            | kFSEventStreamCreateFlagFileEvents
            | kFSEventStreamCreateFlagUseExtendedData
            | kFSEventStreamCreateFlagUseCFTypes,
    )?;

    let mut stream = stream.into_flatten();

    println!("[watcher] FSEvents stream registered successfully",);

    loop {
        tokio::select! {
            // Check shutdown first (biased).
            biased;

            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("[watcher] shutdown signal received, stopping");
                    break;
                }
            }

            event = stream.next() => {
                let event = match event {
                    Some(e) => e,
                    None => {
                        println!("[watcher] FSEvents stream ended");
                        break;
                    }
                };

                if walker::should_ignore_path(&event.path) {
                    continue;
                }

                println!(
                    "[watcher] event {} flags={:x} path={}",
                    event.id,
                    event.raw_flags,
                    event.path.display(),
                );

                if tx.send(event).await.is_err() {
                    eprintln!(
                        "[watcher] indexer channel closed, stopping",
                    );
                    break;
                }
            }
        }
    }

    // `tx` is dropped here → channel closes →
    // indexer will drain remaining events and exit.
    println!("[watcher] stopped");
    Ok(())
}
