use common::*;

use std::path::Path;
use std::borrow::Cow;
use std::{env, fs};
use std::io::ErrorKind::*;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::sync::atomic::{AtomicPtr, Ordering::*};
use std::process::Command;
use std::thread;
extern crate dlopen;
use self::dlopen::raw::Library;
extern crate notify;
use self::notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

fn reload(iterations: &mut usize,  current_size: usize) -> Option<&'static Functions> {
    unsafe {
        let path = match env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Cannot get path of current executable: {}", e);
                return None;
            }
        };
        let mut path = match path.into_os_string().into_string() {
            Ok(path) => path,
            Err(_) => {
                eprintln!("Non-unicode paths are not supported, sorry.");
                return None;
            }
        };
        // (on linux) current_exe() appends " (deleted)" when the file has been replaced
        if path.ends_with(" (deleted)") {
            let len = path.len();
            path.truncate(len-" (deleted)".len());
        }
        // (on linux) dlopen refuses to open the same path multiple times
        let new_name = format!("{}-reload.{}", path, *iterations);
        *iterations += 1;
        if let Err(e) = fs::remove_file(&new_name) {
            if e.kind() != NotFound {
                eprintln!("Cannot delete pre-existing {:?}: {}, trying: .{}",
                    &new_name, e, *iterations+1
                );
                // increments iterations and tries again
                reload(iterations, current_size);
            }
        } else {
            //println!("Deleted pre-existing {:?}", &new_name);
        }
        if let Err(e) = fs::hard_link(&path, &new_name) {
            eprintln!("link {:?} to {:?} failed with {}", path, new_name, e);
            return None;
        }
        drop(path); // prevent using it instead of new_name
        println!("Trying to reload game functions from {:?}", new_name);
        let lib = match Library::open(&new_name) {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Failed to open {:?} as library: {}", new_name, e);
                return None;
            }
        };
        if let Err(e) = fs::remove_file(&new_name) {
            eprintln!("Cannot delete {:?} after creating and loading it: {}", &new_name, e);
        }
        let symbol: Result<&Functions, _> = lib.symbol("GAME");
        match symbol {
            Ok(ref game) if game.size == current_size => {
                // leak the handle because unloading is very risky,
                // this should only happen a limited number of times,
                // and restarting isn't that bad either
                Box::leak(Box::new(lib));
                Some(game)
            }
            Ok(_) => {
                eprintln!("Game struct has changed size, refusing to swap functions");
                None
            }
            Err(_) => {
                eprintln!("{:?} does not have symbol GAME", new_name);
                eprintln!("\tYou need to add `expose_game!{{$GameStruct}}`");
                None
            }
        }
    }
}

fn watch(src: &Path,  functions: &AtomicPtr<Functions>) {
    let (tx, rx) = mpsc::channel();
    let mut watcher: RecommendedWatcher = match Watcher::new(tx, Duration::from_secs(2)) {
        Ok(watcher) => watcher,
        Err(e) => {
            eprintln!("Cannot create fs watcher: {} - Hotswapping will not work", e);
            return;
        }
    };
    if let Err(e) = watcher.watch(&src, RecursiveMode::Recursive) {
        eprintln!("Cannot watch {:?}: {} - Hotswapping will not work", src, e);
        return;
    }
    
    let mut iterations = 1;
    loop {
        let path = match rx.recv() {
            Ok(DebouncedEvent::Write(path)) => path,
            Ok(DebouncedEvent::Error(e, maybe_path)) => {
                eprintln!("fs watch error: {} ({:?})", e, maybe_path);
                continue;
            },
            Ok(_) => continue,
            Err(e) => {
                eprintln!("fs watch error: {}, quitting", e);
                break;
            }
        };
        println!("{:?}", path);
        match Command::new("cargo").arg("build").status() {
            Ok(exit) if exit.success() => {},
            Ok(_) => continue,// cargo printed error
            Err(e) => {
                eprintln!("Failed to start cargo build: {}", e);
                continue;
            }
        }
        let before = unsafe{ &*functions.load(SeqCst) };
        if let Some(new_functions) = reload(&mut iterations, before.size) {
            functions.store(new_functions as *const _ as *mut _, SeqCst);
            let after = new_functions;
            println!("before: mouse_press={:p}->{:p}", before, before.mouse_press);
            println!("after : mouse_press={:p}->{:p}", after, after.mouse_press);
        }
    }
}

pub struct FunctionGetter(Arc<AtomicPtr<Functions>>);
impl FunctionGetter {
    pub fn new(f: Functions,  source_start: Cow<'static,Path>) -> Self {
        let f = Arc::new(AtomicPtr::new(Box::leak(Box::new(f))));
        let clone = f.clone();
        thread::spawn(move|| watch(&source_start, &*clone) );
        FunctionGetter(f)
    }
    pub fn get(&self) -> &Functions {
        unsafe{ &*self.0.load(Acquire) }
    }
}
