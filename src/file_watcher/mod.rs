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

use crate::FileDescription;

mod my_watcher;
pub mod notifications;

use my_watcher::MyWatcher;
pub use notifications::{Notification, UiNotification};

pub struct FileWatcher {
    should_run_tx: Sender<Notification>,
    should_run_rx: Option<Receiver<Notification>>,
    file_descriptions_map: HashMap<PathBuf, FileDescription>,
    ui_notification_transmitters: Vec<Sender<UiNotification>>,
    join_handle: Option<JoinHandle<()>>,
    use_polling: bool,
}

impl FileWatcher {
    pub fn new(file_descriptions: Vec<FileDescription>, use_polling: bool) -> FileWatcher {
        let mut map: HashMap<PathBuf, FileDescription> = HashMap::new();
        for fd in file_descriptions {
            map.insert(fd.from.clone(), fd);
        }
        let (tx, rx): (Sender<Notification>, Receiver<Notification>) = channel();
        FileWatcher {
            file_descriptions_map: map,
            join_handle: None,
            should_run_tx: tx,
            should_run_rx: Some(rx),
            ui_notification_transmitters: Vec::new(),
            use_polling: use_polling,
        }
    }

    pub fn stop(mut self) {
        match self.should_run_tx.send(Notification::B(false)) {
            Ok(_) => {}
            Err(e) => {}
        }
        match self.join_handle {
            Some(jh) => {
                self.join_handle = None;
                println!("Waiting for join handle");
                jh.join();
            }
            None => {}
        }
    }

    pub fn get_ui_notification_receiver(&mut self) -> Receiver<UiNotification> {
        let (tx, rx): (Sender<UiNotification>, Receiver<UiNotification>) = channel();
        self.ui_notification_transmitters.push(tx);
        rx
    }

    fn send_ui_notification(
        transmitters: &Vec<Sender<UiNotification>>,
        notification: UiNotification,
    ) {
        //println!("Sending a notification");
        for tx in transmitters {
            tx.send(notification.clone());
        }
    }

    fn get_watcher(
        tx: Sender<DebouncedEvent>,
        use_polling: bool,
    ) -> Result<MyWatcher, notify::Error> {
        match use_polling {
            true => match PollWatcher::new(tx, Duration::from_millis(100)) {
                Ok(v) => Ok(MyWatcher::Pw(v)),
                Err(e) => {
                    println!("{:?}", e);
                    Err(e)
                }
            },
            false => match watcher(tx, Duration::from_millis(100)) {
                Ok(v) => Ok(MyWatcher::Rdcw(v)),
                Err(e) => {
                    println!("{:?}", e);
                    Err(e)
                }
            },
        }
    }

    pub fn start(mut self) -> FileWatcher {
        let mut file_descriptions_map_clone: HashMap<PathBuf, FileDescription> = HashMap::new();
        for fd in self.file_descriptions_map.values() {
            file_descriptions_map_clone.insert(fd.from.clone(), fd.clone());
        }

        let (tx, rx) = channel();

        let tx_pass_through = self.should_run_tx.clone();
        let rx_combined = self.should_run_rx.unwrap();
        self.should_run_rx = None;

        // thread to pack the debounced event.
        thread::spawn(move || loop {
            match rx.recv() {
                Ok(v) => {
                    tx_pass_through.send(Notification::E(v));
                }
                Err(e) => {}
            }
        });

        let transmitters: Vec<Sender<UiNotification>> =
            self.ui_notification_transmitters.iter().cloned().collect();

        let use_polling = self.use_polling.clone();

        let jh: JoinHandle<()> = thread::spawn(move || {
            // Create a channel to receive the events.

            // Automatically select the best implementation for your platform.
            // You can also access each implementation directly e.g. INotifyWatcher.

            let mut watch = match FileWatcher::get_watcher(tx, use_polling) {
                Ok(v) => v,
                Err(e) => {
                    return;
                }
            };
            FileWatcher::send_ui_notification(&transmitters, UiNotification::Started);

            let mut watched_files: Vec<PathBuf> = Vec::new();

            for fd in file_descriptions_map_clone.values() {
                let from = fd.from.clone();
                match watch.watch(&from, RecursiveMode::NonRecursive) {
                    Ok(_) => {
                        watched_files.push(from.clone());
                        FileWatcher::send_ui_notification(
                            &transmitters,
                            UiNotification::StartedWatching(from.clone()),
                        );
                    }
                    Err(e) => {
                        println!("Couldn't watch file: {:?} because {:?}", "", e);
                    }
                };
            }

            loop {
                // Receive
                match rx_combined.recv() {
                    Ok(event) => match event {
                        Notification::E(ev) => match ev {
                            NoticeWrite(p) => {
                                //println!("{:?}", p);
                                match file_descriptions_map_clone.get(&p) {
                                    Some(fd) => {
                                        //println!("Found key!");
                                        // Notify
                                        FileWatcher::send_ui_notification(
                                            &transmitters,
                                            UiNotification::Copied(fd.from.clone(), fd.to.clone()),
                                        );
                                        fd.copy();
                                    }
                                    None => {
                                        println!("Could not find key");
                                    }
                                }
                                /*
                                for pb in file_descriptions_map_clone.keys() {
                                    println!("{:?}", pb);
                                }
                                */
                            }
                            _ => {}
                        },
                        Notification::B(b) => {
                            if b == false {
                                //println!("Started with unwatching");
                                for p in watched_files {
                                    //println!("Unwatching: {:?}", &p);
                                    let p_clone = p.clone();
                                    match watch.unwatch(p) {
                                        Ok(_) => {
                                            //println!("unwatched");
                                            FileWatcher::send_ui_notification(
                                                &transmitters,
                                                UiNotification::StoppedWatching(p_clone),
                                            );
                                        }
                                        Err(e) => {
                                            println!(
                                                "Could not stop watching {:?} because {:?}",
                                                "", e
                                            );
                                        }
                                    };
                                }
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        //Disconnected
                        //println!("watch error: {:?}", e);
                    }
                }
            }
        });
        self.join_handle = Some(jh);
        self
    }
}
