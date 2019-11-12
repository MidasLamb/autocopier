use crate::file_watcher::notifications::UiNotification;
use chrono::offset::Local;
use chrono::DateTime;
use console::style;
use console::Term;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::{fs, thread};

pub trait Ui {
    fn start(
        notification_rx: Receiver<UiNotification>,
        termination_tx: Sender<()>,
    ) -> JoinHandle<()>;
}

pub struct Tui {}

struct TuiInformation {
    map: HashMap<PathBuf, Option<SystemTime>>,
    last_output: String,
}

impl Tui {
    fn redraw_screen(term: Term, info: &mut TuiInformation) {
        let mut to_output: String = String::new();
        for (key, val) in &info.map {
            match val {
                Some(st) => {
                    if (SystemTime::now().duration_since(*st).unwrap() < Duration::from_secs(10)) {
                        to_output.push_str(&format!(
                            "{} (just now)",
                            style(key.to_str().unwrap()).green()
                        ));
                    } else if (SystemTime::now().duration_since(*st).unwrap()
                        < Duration::from_secs(60))
                    {
                        to_output.push_str(&format!(
                            "{} (< 1 minute)",
                            style(key.to_str().unwrap()).yellow()
                        ));
                    } else if (SystemTime::now().duration_since(*st).unwrap()
                        < Duration::from_secs(10 * 60))
                    {
                        to_output.push_str(&format!(
                            "{} (< 10 minutes)",
                            style(key.to_str().unwrap()).blue()
                        ));
                    } else {
                        to_output.push_str(&format!(
                            "{} (> 10 minutes)",
                            style(key.to_str().unwrap()).white()
                        ));
                    }
                }
                None => {
                    to_output.push_str(&format!(
                        "{} (not yet)",
                        style(key.to_str().unwrap()).white()
                    ));
                }
            }
            to_output.push_str("\r\n");
        }
        to_output.push_str("Press q to stop.\r\n");

        if (to_output != info.last_output) {
            term.clear_screen();
            term.write_line(&to_output);
            info.last_output = to_output;
        }
    }
}

impl Ui for Tui {
    fn start(
        notification_rx: Receiver<UiNotification>,
        termination_tx: Sender<()>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let term = Term::stdout();
            term.clear_line();
            let term_clone = term.clone();

            thread::spawn(move || {
                //block and wait for char.;
                loop {
                    match term_clone.read_char() {
                        Ok('q') => {
                            termination_tx.send(());
                            break;
                        }
                        _ => {
                            term_clone.write_line("Not recognized.");
                        }
                    }
                }
            });

            // TuiInformation
            let mut tui_information: TuiInformation = TuiInformation {
                map: HashMap::new(),
                last_output: String::from(""),
            };

            loop {
                match notification_rx.try_recv() {
                    Ok(UiNotification::Started) => {
                        println!("We started!");
                    }
                    Ok(UiNotification::StartedWatching(pb)) => {
                        println!("Started watching {:?}", pb);
                        tui_information.map.insert(pb, None);
                    }
                    Ok(UiNotification::StoppedWatching(pb)) => {
                        println!("Stopped watching {:?}", pb);
                        tui_information.map.remove(&pb);
                    }
                    Ok(UiNotification::Copied(from, to)) => {
                        println!("Copied {:?} to {:?}", from, to);
                        tui_information.map.insert(from, Some(SystemTime::now()));
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        // Disconnected, so we just stop.
                        println!("Disconnected");
                        break;
                    }
                }
                Tui::redraw_screen(term.clone(), &mut tui_information);
                thread::sleep(Duration::from_millis(250));
            }
        })
    }
}
