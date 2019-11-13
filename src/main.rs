extern crate clap;

extern crate chrono;
extern crate console;
extern crate ctrlc;
extern crate notify;
extern crate serde;
extern crate serde_json;

mod configuration_reader;
mod file_watcher;
mod ui;

use configuration_reader::*;
use console::Term;
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time;
use std::time::Duration;

use file_watcher::FileWatcher;
use ui::{Tui, Ui};

use clap::{App, Arg};

#[derive(Debug)]
pub struct FileDescription {
    pub from: PathBuf,
    pub to: PathBuf,
}

pub enum StepInChain {
    Start,
    End,
}

impl FileDescription {
    pub fn copy(&self) {
        let mut attempts: u32 = 0;
        while attempts < 100 {
            println!("Copy from {:?} to {:?}!", &self.from, &self.to);
            match fs::copy(&self.from, &self.to) {
                Ok(_) => {
                    println!("Copy: Break!");
                    break;
                }
                Err(e) => {
                    println!("{:?}", e);
                    attempts += 1;
                    thread::sleep(Duration::from_millis(10));
                }
            };
        }
    }
}

impl Clone for FileDescription {
    fn clone(&self) -> FileDescription {
        FileDescription {
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    // Set up flags
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("configurationfile")
                .short("f")
                .long("file")
                .takes_value(true)
                .help("The configuration file, in json format. Defaults to configuration.json."),
        )
        .arg(
            Arg::with_name("step")
                .short("s")
                .long("step")
                .takes_value(true)
                .help("The step in the copy chain. Possible values are 'start' and 'end'. Defaults to 'end'."),
        )
        .arg(
            Arg::with_name("use_polling")
                .short("p")
                .long("use_polling")
                .help("Use polling instead of using the OS events."),
        )
        .arg(
            Arg::with_name("print_watched")
                .short("w")
                .long("print_watched")
                .help("Print the files that will be watched and exit."),
        )
        .get_matches();

    let configuration_file = matches
        .value_of("configurationfile")
        .unwrap_or("configuration.json");
    let step_in_chain_str: &str = &matches.value_of("step").unwrap_or("end").to_lowercase();
    let step_in_chain: StepInChain = match step_in_chain_str {
        "start" => StepInChain::Start,
        "end" => StepInChain::End,
        e => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Step in chain was not recognised, get {}", e),
            ))
        }
    };

    let use_polling: bool = matches.is_present("use_polling");
    let print_watched: bool = matches.is_present("print_watched");

    // Parse configuration
    let (mut configuration, unparsed) = match parse_configuration(configuration_file, step_in_chain)
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Could not parse configuration, because: {:?}", e);
            let mut input = String::new();
            eprintln!("Press enter to continue.");
            std::io::stdin().read_line(&mut input)?;
            return Err(e);
        }
    };

    if print_watched {
        for fd in configuration.files {
            println!("{:?} \r\n\tto {:?}", fd.from, fd.to);
        }
        return Ok(());
    }

    // Set up channel for termination.
    let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
    let tx_ctrlc = tx.clone();
    let tx_ui = tx.clone();

    // Set up filewatcher and ui.
    let mut file_watcher: FileWatcher = FileWatcher::new(configuration.files, use_polling);
    let ui_jh = Tui::start(file_watcher.get_ui_notification_receiver(), tx_ui);

    file_watcher = file_watcher.start();

    ctrlc::set_handler(move || {
        tx_ctrlc.send(());
    });

    match rx.recv() {
        Ok(()) => {
            file_watcher.stop();
            println!("stopped ok");
            ui_jh.join();
            return Ok(());
        }
        Err(_) => {
            return Ok(());
        }
    }
}
