use notify::DebouncedEvent::*;
use notify::RecursiveMode::Recursive;
use notify::{
    watcher, DebouncedEvent, PollWatcher, ReadDirectoryChangesWatcher, RecursiveMode, Watcher,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::{fs, thread};

/// Represents all possible file watchers.
pub enum MyWatcher {
    Pw(PollWatcher),
    MyPw(PollingWatcher),
    Rdcw(ReadDirectoryChangesWatcher),
}

impl MyWatcher {
    pub fn transform_path<P: AsRef<Path>>(path: P) -> PathBuf {
        if path.as_ref().is_absolute() {
            path.as_ref().to_owned()
        } else {
            let p = env::current_dir().unwrap();
            p.join(path)
        }
    }

    /// Creates a watcher, either a polling one or one that asks the OS to notify us of events.
    pub fn get_watcher(
        tx: Sender<DebouncedEvent>,
        use_polling: bool,
    ) -> Result<MyWatcher, notify::Error> {
        match use_polling {
            true => match PollingWatcher::new(tx, Duration::from_millis(100)) {
                Ok(v) => Ok(MyWatcher::MyPw(v)),
                Err(e) => {
                    println!("{:?}", e);
                    Err(e)?
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

    pub fn watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        recursive_mode: RecursiveMode,
    ) -> Result<(), notify::Error> {
        match self {
            MyWatcher::Pw(w) => w.watch(path, recursive_mode),
            MyWatcher::MyPw(w) => w.watch(path, recursive_mode),
            MyWatcher::Rdcw(w) => w.watch(path, recursive_mode),
        }
    }

    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        match self {
            MyWatcher::Pw(w) => w.unwatch(path),
            MyWatcher::MyPw(w) => w.unwatch(path),
            MyWatcher::Rdcw(w) => w.unwatch(path),
        }
    }
}

struct PollingWatcher {
    files_to_watch: Arc<Mutex<HashSet<PathBuf>>>,
    watcher_thread: JoinHandle<()>,
}

impl PollingWatcher {
    fn new(tx: Sender<DebouncedEvent>, dur: Duration) -> Result<PollingWatcher, notify::Error> {
        let files_to_watch_vec: HashSet<PathBuf> = HashSet::new();
        let files_to_watch = Arc::new(Mutex::new(files_to_watch_vec));
        let files_to_watch_clone = files_to_watch.clone();

        let tx_clone = tx.clone();
        let jh = thread::spawn(move || {
            let mut hm: HashMap<PathBuf, SystemTime> = HashMap::new();
            loop {
                for p in &*files_to_watch_clone.lock().unwrap() {
                    let metadata = match fs::metadata(&p) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let modified_since = metadata.modified().unwrap();
                    if hm.contains_key(p) {
                        if modified_since > *hm.get(p).unwrap() {
                            tx_clone.send(NoticeWrite(p.clone()));
                            hm.insert(p.clone(), modified_since);
                        }
                    } else {
                        hm.insert(p.clone(), modified_since);
                    }
                }
                thread::sleep_ms(100);
            }
        });
        Ok(PollingWatcher {
            files_to_watch: files_to_watch,
            watcher_thread: jh,
        })
    }

    fn watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        recursive_mode: RecursiveMode,
    ) -> Result<(), notify::Error> {
        let mut v = &mut *self.files_to_watch.lock().unwrap();
        v.insert(MyWatcher::transform_path(path));
        Ok(())
    }

    fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let mut v = &mut *self.files_to_watch.lock().unwrap();
        v.remove(&path.as_ref().to_path_buf());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn test_watcher(mut pw: MyWatcher, rx: Receiver<DebouncedEvent>, filename: &str) {
        let mut f = File::create(filename).unwrap();
        f.sync_all();
        pw.watch(filename, RecursiveMode::NonRecursive);
        thread::sleep(Duration::from_secs(1));

        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        f.write_all(b"Test");
        f.sync_all();
        drop(f);

        thread::sleep(Duration::from_secs(1));
        fs::remove_file(filename);

        assert_eq!(
            rx.try_recv(),
            Ok(DebouncedEvent::NoticeWrite(MyWatcher::transform_path(
                filename
            )))
        );
    }

    #[test]
    fn test_polling_watcher() {
        let (tx, rx) = channel();
        let pw = MyWatcher::get_watcher(tx, true).unwrap();
        test_watcher(pw, rx, "polling_watcher_test.txt");
    }

    #[test]
    fn test_os_watcher() {
        let (tx, rx) = channel();
        let pw = MyWatcher::get_watcher(tx, false).unwrap();
        test_watcher(pw, rx, "os_watcher_text.txt");
    }
}
