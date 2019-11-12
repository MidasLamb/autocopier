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

pub enum Notification {
    E(DebouncedEvent),
    B(bool),
}

pub enum UiNotification {
    Started,
    StartedWatching(PathBuf),
    StoppedWatching(PathBuf),
    Copied(PathBuf, PathBuf),
}

impl Clone for UiNotification {
    fn clone(&self) -> UiNotification {
        match self {
            UiNotification::Started => UiNotification::Started,
            UiNotification::StartedWatching(pb) => UiNotification::StartedWatching(pb.clone()),
            UiNotification::StoppedWatching(pb) => UiNotification::StoppedWatching(pb.clone()),
            UiNotification::Copied(from, to) => UiNotification::Copied(from.clone(), to.clone()),
        }
    }
}
