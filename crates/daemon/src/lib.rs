pub mod indexed;
pub mod start;
pub mod watcher;

// Re-export the event type so consumers don't need a
// direct fsevent-stream dependency.
pub use fsevent_stream::stream::Event as FsEvent;

// Re-export the top-level start/shutdown for convenience.
pub use start::shutdown;
pub use start::start;
