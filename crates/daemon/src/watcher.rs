use crossbeam_channel::Sender;

use notify::{
    Config,
    Event,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};

use std::path::Path;

pub fn start_watcher(
    root: &str,
    tx: Sender<Event>,
) -> notify::Result<()> {
    println!(
        "[watcher] starting recursive watch on {}",
        root,
    );

    let mut watcher =
        RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                match res {
                    Ok(event) => {
                        println!(
                            "[watcher] event {:?} ({} paths)",
                            event.kind,
                            event.paths.len(),
                        );
                        if let Err(err) =
                            tx.send(event)
                        {
                            eprintln!(
                                "Failed to forward fs event: {}",
                                err,
                            );
                        }
                    }

                    Err(err) => {
                        eprintln!(
                            "Watcher error: {}",
                            err,
                        );
                    }
                }
            },
            Config::default(),
        )?;

    watcher.watch(
        Path::new(root),
        RecursiveMode::Recursive,
    )?;

    println!(
        "[watcher] watch registered successfully",
    );

    loop {
        std::thread::park();
    }
}
