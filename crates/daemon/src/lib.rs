pub mod watcher;
pub mod indexed;
pub mod start;

// Re-export the event type so consumers don't need a
// direct fsevent-stream dependency.
pub use fsevent_stream::stream::Event as FsEvent;

// Re-export the top-level start function for convenience.
pub use start::start;