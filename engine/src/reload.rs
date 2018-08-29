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

fn reload(iterations: &mut usize) -> Option<Functions> {
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
                    &new_name, e, *iterations
                );
                reload(iterations);
            }
        } else {
            println!("Deleted pre-existing {:?}", &new_name);
        }
        if let Err(e) = fs::hard_link(&path, &new_name) {
            eprintln!("link {:?} to {:?} failed with {}", path, new_name, e);
            return None;
        }
        println!("Trying to reload game functions from {:?}", new_name);
        let lib = match Library::open(&new_name) {
            // leak the handle because unloading is very risky,
            // this should only happen a limited number of times,
            // and restarting isn't that bad either
            Ok(lib) => Box::leak(Box::new(lib)),
            Err(e) => {
                eprintln!("Failed to open {:?} as library: {}", new_name, e);
                return None;
            }
        };
        if let Err(e) = fs::remove_file(&new_name) {
            eprintln!("Cannot delete {:?} after creating and loading it: {}", &new_name, e);
        }
        let functions = (
            lib.symbol("game_render"),
            lib.symbol("game_update"),
            lib.symbol("game_mouse_move"),
            lib.symbol("game_mouse_press"),
        );
        match functions {
            (Ok(render), Ok(update), Ok(mouse_move), Ok(mouse_press)) => {
                Some(Functions{render, update, mouse_move, mouse_press})
            }
            (Err(ref e),_,_,_) | (_,Err(ref e),_,_) | (_,_,Err(ref e),_) | (_,_,_,Err(ref e)) => {
                eprintln!("{:?} is missing symbol(s): {}", path, e);
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
            eprintln!("Cannot create fs watcher: {}\nLive reload will not be supported", e);
            return;
        }
    };
    if let Err(e) = watcher.watch(&src, RecursiveMode::Recursive) {
        eprintln!("Cannot watch {:?}: {}\nLive reload will not be supported", src, e);
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
        if let Some(new_functions) = reload(&mut iterations) {
            let after = Box::leak(Box::new(new_functions));
            let before = functions.swap(after, SeqCst);
            let before = unsafe{&*before};
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
