use std::path::Path;
use std::time::Duration;

use fsevent_stream::ffi::{
    kFSEventStreamCreateFlagFileEvents,
    kFSEventStreamCreateFlagNoDefer,
    kFSEventStreamCreateFlagUseCFTypes,
    kFSEventStreamCreateFlagUseExtendedData,
    kFSEventStreamEventIdSinceNow,
    FSEventStreamEventId,
};
use fsevent_stream::stream::{
    create_event_stream, Event,
};
use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;

use blaze_core::walker;

/// Sentinel value: start streaming from "now", ignoring
/// all historical events.
pub const SINCE_NOW: FSEventStreamEventId =
    kFSEventStreamEventIdSinceNow;

/// Start an FSEvents stream on `root` that replays every
/// event whose ID > `since`, then continues in real-time.
///
/// Events for ignored paths (see `walker::should_ignore_path`)
/// are silently dropped before they reach the channel.
pub async fn start_watcher(
    root: &str,
    since: FSEventStreamEventId,
    tx: Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
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

    println!(
        "[watcher] FSEvents stream registered successfully",
    );

    while let Some(event) = stream.next().await {
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

    Ok(())
}
