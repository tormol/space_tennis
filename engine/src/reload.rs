use common::*;

use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::{env, fs};
use std::io::{self, Write, ErrorKind::*};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering::*};
use std::process::Command;
use std::thread;
extern crate dlopen;
use self::dlopen::raw::Library;
extern crate notify;
use self::notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
extern crate serde_json;

/*
using rustc isn't working - the build plan is missing ["-L", "native=/home/tbm/p/rust/space_tennis/target/debug/build/libloading-d2593e82793bee08/out"]
(which cargo build --verbose shows)
and target/debug/space_tennis isn't modified
I think it has something to do with rustup, ive set override nightly, so that all invocations are at least using the same version (or?)
somehow the hard-link-to-reload doesn't work either (loads the same address)
    it works once (because looking in different dir?)
it works with cargo build, repeatedly
strace creates too much noise

curren theory: i think rustc puts the exe in target/debug/deps/
and that cargo copies it into target/debug
adding -o current_exe() conflicts with --out-dir .../deps/
(warning) and doesn't fix it completely:
    an -ID is appended to the exe (from -C extra-filename= or -C metadata=)
Solution (i hope): inv.get("outputs")?.as_array()?.first()?
["links"] might be even better as it has the target name too
*/

fn run_custom_build_steps(build_plan: &serde_json::Value) -> Option<Vec<String>> {
    let mut more_args = Vec::new();
    for step in build_plan.get("invocations")?.as_array()? {
        if step.get("target_kind")?.as_array()?.first()?.as_str()? != "custom-build" {
            continue;
        }
        if step.get("program")?.as_str()? == "rustc" {
            continue; // the compile step for build.rs
        }
        println!("Running custom build step for {}", step.get("package_name")?.as_str()?);
        let program = step.get("program")?.as_str()?;
        let args = step.get("args")?.as_array()?.iter()
            .map(|value| value.as_str() ).collect::<Option<Vec<_>>>()?;
        let envs = step.get("env")?.as_object()?.iter()
            .map(|(var,v)| v.as_str().map(|value| (var,value) ) )
            .collect::<Option<Vec<_>>>()?;
        let dir = step.get("cwd").and_then(|v| v.as_str() )?;
        let mut command = Command::new(program);
        command.current_dir(dir);
        command.args(args);
        command.envs(envs);
        let output = command.output().ok()?;
        if !output.status.success() {
            eprintln!("\t\"{}\" failed with exit code {:?}:", program, output.status);
            let _ = io::stderr().write(&output.stderr);
            continue;
        }
        let stdout = String::from_utf8(output.stdout).ok()?;
        for line in stdout.lines() {
            println!("\t{}", line);
            if let Some(split) = line.find('=') {
                let k = line[..split].trim();
                if k == "cargo:rustc-link-search" {
                    let v = line[split+1..].trim();
                    println!("\tAdding \"-L\" \"{}\"", v);
                    more_args.push("-L".to_string());
                    more_args.push(v.to_string());
                }
            }
        }
    }
    Some(more_args)
}

/// Turns the equivalent of `jq '.invocations | .[-1] | {env,cwd,program,args,outputs}'`
/// (from --build-plan JSON) into a Command, and also gets the source code root from args.
fn extract_final_rustc(build_plan: &serde_json::Value) -> Option<(PathBuf,Command,String,String)> {
    // build_plan.invocations.[-1].args: [str] is the only required field
    // print warnings when the other fields are missing or malformed
    let inv = build_plan.get("invocations")?.as_array()?.last()?.as_object()?;
    // contains the path of the output executable
    let links = inv.get("links")?.as_object()?;
    if links.len() > 1 {
        eprintln!("Final build step has multiple outputs: {:?}, choosing {:?}",
            links, links.iter().next().unwrap()
        );
    }
    let (link, exe) = links.into_iter().next()
        .and_then(|(link,v)| v.as_str().map(|exe| (link.to_string(),exe.to_string()) ) )?;
    // collect the arguments to search for program root in them after passing
    // to .args(). Collecting options into an option short-circuits.
    let args = inv.get("args")?.as_array()?
        .iter().map(|value| value.as_str() ).collect::<Option<Vec<&str>>>()?;

    let program = inv.get("program").and_then(|v| v.as_str() ).unwrap_or_else(|| {
        eprintln!("Build plan step is missing \"program\" (or it's not a string), assuming `rustc`");
        "rustc"
    });
    let mut command = Command::new(program);
    command.args(&args);

    if let Some(wd) = inv.get("cwd").and_then(|v| v.as_str() ) {
        command.current_dir(wd);
    } else {
        eprintln!("Build plan step is missing \"cwd\" (or it's not a string), assuming the current one is OK");
    }

    if let Some(env) = inv.get("env").and_then(|v| v.as_object() ) {
        for (k,v) in env {
            // could have used .envs(...filter_map()) but I don't want to
            // swallow any unexpectancies
            if let Some(v) = v.as_str() {
                command.env(k, v);
            } else {
                eprintln!("Build plan has non-string environment variable \"{}\", skipping it", k);
            }
        }
    } else {
        eprintln!("Build plan step is missing \"env\" (or it's not an array), trying without");
    }

    // Required to expose statics in executables, adding it here
    // avoids requiring a .cargo/config
    // currently set in .cargo/config, as it works better
    // adding --verbose doesn't make it print anything
    command.args(&["-C","link-args=-Wl,-export-dynamic"]);
    // --build-plan forgets this
    //command.args(&["-L","native=/home/tbm/p/rust/space_tennis/target/debug/build/libloading-d2593e82793bee08/out"]);

    // If the program root is inside a subdirectory (like src/), watch the
    // entire directoy, otherwise, assume it's a single file and only watch it.
    // Use Path to split directories properly. The path is guaranteed to be
    // unicode because cargo fails otherwise.
    let watch = match args.iter().find(|arg| arg.ends_with(".rs") ) {
        Some(main) => {
            let path = Path::new(main);
            path.parent().filter(|&p| p != Path::new("") ).unwrap_or(path).to_owned()
        }
        None => {
            eprintln!("no .rs file in {} invocation, falling back to watching src/", program);
            PathBuf::from("src")
        }
    };
    Some((watch,command,exe,link))
}

/// Fallback when getting or parsing a build plan fails
fn default_cargo(cargo_args: &[OsString]) -> (PathBuf,Command,String,String) {
    eprintln!("falling back to using `cargo build` and looking for changes in src/");
    let mut cargo = Command::new("cargo");
    cargo.arg("build");
    //cargo.arg("--verbose");
    cargo.args(cargo_args);
    // see extract_final_rustc()
    cargo.env("RUSTFLAGS", "-C link-args=-Wl,-export-dynamic");
    (PathBuf::from("src"), cargo, get_exe().unwrap(), get_exe().unwrap())
}


/// Get command to compile the final executable assuming unchanged dependencies
/// Tries to extract a command from a nightly cargo --build-plan, but falls
/// back to the slower `cargo build` on errors.
/// Prints errors and warnings to stderr
///
/// Does not try to use rustup to get nightly, because
/// a) it might not be installed, and having two paths complicate the code.
/// b) would not allow people to use a specific nightly.
/// c) build plan has toolchain-specific --incremental arguments and .rlib IDs,
///    which means you need to build with the same (nightly) toolchain,
///    people need to build the initial binary as nightly as loading a library
///    compiled with different compiler is a bad idea.
///
/// FIXME: try without "-Zunstable-options", (but that complicates the code)
fn get_compile_command(cargo_args: &[OsString]) -> Option<(PathBuf,Command,String,String)> {
    let output = Command::new("cargo")
        .args(&["build","-Zunstable-options","--build-plan"])
        .args(cargo_args)
        .output();
    let stdout = match output {
        Err(e) => {
            eprintln!("Cannot run cargo: {}, giving up reloading", &e);
            return None;
        }
        Ok(ref output) if !output.status.success() => {
            eprintln!("`cargo --build-plan` failed (");
            let _ = io::stderr().write(&output.stderr);
            eprint!("), --build-plan requires a nightly toolchain, ");
            eprintln!("You might want to run `rustup override set nightly`.");
            return Some(default_cargo(cargo_args));
        }
        Ok(output) => {
            let _ = io::stderr().write(&output.stderr);
            output.stdout
        }
    };
    let json = match serde_json::from_slice::<serde_json::Value>(&stdout) {
        Ok(json) => json,
        Err(_) => {
            eprint!("Build plan is not JSON, ");
            return Some(default_cargo(cargo_args));
        }
    };
    if let Some((src, mut command, exe, link)) = extract_final_rustc(&json) {
        match run_custom_build_steps(&json) {
            Some(args) => {command.args(args);}
            None => {eprintln!("Parsing custom build steps failed, trying without");}
        }
        Some((src,command,exe,link))
    } else {
        eprint!("Could not get rustc command from build plan, ");
        Some(default_cargo(cargo_args))
    }
}

fn get_exe() -> Option<String> {
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
    Some(path)
}

fn reload(exe: &str,  current_size: usize) -> Option<&'static Functions> {
    static ITERATIONS: AtomicUsize = AtomicUsize::new(1);
    // (on linux) dlopen refuses to open the same path multiple times
    let new_name = loop {
        let iterations = ITERATIONS.fetch_add(1, Relaxed);
        let new_name = format!("{}-reload.{}", exe, iterations);
        if let Err(e) = fs::remove_file(&new_name) {
            if e.kind() != NotFound {
                eprintln!("Cannot delete pre-existing {:?}: {}, trying: .{}",
                    &new_name, e, iterations+1
                );
                continue; // increase iterations and try again
            }
        } else {
            //println!("Deleted pre-existing {:?}", &new_name);
        }
        break new_name;
    };
    if let Err(e) = fs::hard_link(exe, &new_name) {
        eprintln!("Cannot create link to {:?} at {:?}: {}", exe, new_name, e);
        return None;
    }
    drop(exe); // prevent using it instead of new_name
    println!("Trying to reload game functions from {:?}", new_name);
    unsafe {
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

fn watch(src: &Path,  callback: &mut dyn FnMut(PathBuf)) {
    'rewatch: loop {
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

        loop {
            match rx.recv() {
                Ok(DebouncedEvent::Write(path)) => callback(path),
                Ok(DebouncedEvent::Remove(path)) => {
                    // gedit saves files by deleting it and then creating the modified version.
                    // when the watchpoint is removed the watcher will not receive new events,
                    // so we need to restart it.
                    // an alternative solution would be to always watch a directory, but then we would get events
                    // for target/ too and would need to filter.
                    // path is absolute, so check if it ends
                    if path.ends_with(src) {
                        callback(path); // hope the file is saved again before the compiler reads it
                        continue 'rewatch;
                    }
                }
                Ok(DebouncedEvent::Error(e, maybe_path)) => {
                    eprintln!("fs watch error: {} ({:?})", e, maybe_path);
                }
                Err(e) => {
                    eprintln!("fs watch error: {}, quitting", e);
                    return;
                }
                //Ok(e) => println!("other watch event: {:?}", e)
                //Ok(_) => {}
            }
        }
    }
}

pub struct FunctionGetter(Arc<AtomicPtr<Functions>>);
impl FunctionGetter {
    #[inline(never)]
    pub fn new(f: Functions,  cargo_args: Vec<OsString>) -> Self {
        let f = Arc::new(AtomicPtr::new(Box::leak(Box::new(f))));
        let f_clone = f.clone();
        thread::spawn(move|| {
            let functions = f_clone;
            // Don't delay game start on getting the build plan
            if let Some((src, mut command, exe, link)) = get_compile_command(&cargo_args) {
                //command = default_cargo(&cargo_args).1;
                println!("Watching {:?} for source code changes", &src);
                println!("command: {:?}", &command);
                watch(&src, &mut|_| {
                    match command.status() {// runs the command
                        Ok(exit) if exit.success() => {},
                        Ok(_) => return,// cargo printed error
                        Err(e) => {
                            eprintln!("Failed to start cargo build: {}", e);
                            return;
                        }
                    }
                    let path = if exe != link {
                        let _ = fs::remove_file(&link);
                        if let Err(e) = fs::hard_link(&exe, &link) {
                            eprintln!("Cannot create link to {:?} at {:?}: {}", &exe, &link, e);
                            &exe
                        } else {&link}
                    } else {&exe};
                    let before = unsafe{ &*functions.load(SeqCst) };
                    if let Some(new_functions) = reload(path, before.size) {
                        functions.store(new_functions as *const _ as *mut _, SeqCst);
                        let after = new_functions;
                        println!("before: mouse_press={:p}->{:p}", before, before.mouse_press);
                        println!("after : mouse_press={:p}->{:p}", after, after.mouse_press);
                    }
                });
            }
        });
        FunctionGetter(f)
    }
    pub fn get(&self) -> &Functions {
        unsafe{ &*self.0.load(Acquire) }
    }
}
