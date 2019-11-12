use notify::DebouncedEvent::*;
use notify::RecursiveMode::Recursive;
use notify::{
    watcher, DebouncedEvent, PollWatcher, ReadDirectoryChangesWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fs, thread};

pub enum MyWatcher {
    Pw(PollWatcher),
    Rdcw(ReadDirectoryChangesWatcher),
}
impl MyWatcher {
    pub fn watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        recursive_mode: RecursiveMode,
    ) -> Result<(), notify::Error> {
        match self {
            MyWatcher::Pw(w) => w.watch(path, recursive_mode),
            MyWatcher::Rdcw(w) => w.watch(path, recursive_mode),
        }
    }

    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        match self {
            MyWatcher::Pw(w) => w.unwatch(path),
            MyWatcher::Rdcw(w) => w.unwatch(path),
        }
    }
    // similarly for other functions you need
}
