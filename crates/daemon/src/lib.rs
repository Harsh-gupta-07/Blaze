pub mod watcher;
pub mod indexed;

// Re-export the event type so consumers (main.rs, tauri)
// don't need a direct fsevent-stream dependency.
pub use fsevent_stream::stream::Event as FsEvent;