use common::*;

use std::borrow::Cow;
use std::collections::HashMap;
use std::{env, fs};
use std::io::{self, Write, ErrorKind::*};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering::*};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, SystemTime};
use dlopen::raw::Library;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

/// A std::process::Command that can be (de)serialized
///
/// Doesn't store any info about what to do with the streams either.
///
/// Uses str instead of OsStr/Path becaue serde doesn't handle those correctly,
/// and cargo rejects non-UTF8 paths and arguments.
#[derive(Serialize,Deserialize, Clone)]
struct SerializableCommand {
    program: String,
    args: Vec<Box<str>>,
    env: HashMap<Box<str>,Box<str>>,
    cwd: Option<Box<str>>,
}
impl SerializableCommand {
    pub fn new(name: &str) -> Self {
        SerializableCommand {
            program: name.into(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None
        }
    }
    pub fn args<A:AsRef<str>, I:IntoIterator<Item=A>>
    (&mut self,  args: I) -> &mut Self {
        self.args.extend(args.into_iter().map(|a| a.as_ref().into() ));
        return self;
    }
    pub fn env(&mut self,  k: &str,  v: &str) -> &mut Self {
        self.env.insert(k.into(), v.into());
        return self;
    }
    pub fn current_dir(&mut self,  dir: &str) -> &mut Self {
        self.cwd = Some(dir.into());
        return self;
    }
    pub fn create(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(self.args.iter().map(|a| a.as_ref() ));
        cmd.envs(self.env.iter().map(|(k,v)| (k.as_ref(), v.as_ref()) ));
        if let Some(ref dir) = self.cwd {
            cmd.current_dir(dir.as_ref());
        }
        cmd
    }
}


/// Information that is used to invalidate the cached rustc commmand
#[derive(Serialize,Deserialize, PartialEq,Eq, Debug, Clone)]
struct BuildState<'a> {
    lock_timestamp: SystemTime,
    compiler_version: String,
    cargo_args: Cow<'a,[Box<str>]>,
}
impl<'a> BuildState<'a> {
    fn current(cargo_args: &'a[Box<str>]) -> Option<Self> {
        let lock_timestamp = match fs::metadata("Cargo.lock").and_then(|m| m.modified() ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Cannot save cache: Cannot get timestamp for Cargo.lock: {}", e);
                return None;
            }
        };
        let compiler_version = match Command::new("rustc").arg("--version").output() {
            // ignore exit status; --version is unlikely to not work
            Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),// there is no allocation-reusing
            Err(e) => {
                eprintln!("Cannot cache rustc command: Cannot run `rustc`: {}", e);
                return None;
            }
        };
        let cargo_args = Cow::Borrowed(cargo_args);
        Some(BuildState { lock_timestamp, compiler_version, cargo_args })
    }
}

/// Everything that is needed for watching for changes and reloading
#[derive(Serialize,Deserialize, Clone)]// Clone required by Cow, but is never useed
struct BuildInfo {
    // using Arc requires enabling serde's "rc" feature
    command: SerializableCommand,
    src_root: String,
    exe: String,
    link: String,
}

/// What is saved to disk.
///
/// The types have been designed to avoid invalid states after successful deserialization.
#[derive(Serialize,Deserialize)]
struct CacheInfo<'a> {
    check: Cow<'a,BuildState<'a>>,
    build: Cow<'a,BuildInfo>,
}
impl<'a> CacheInfo<'a> {
    const FILE: &'static str = "target/reload-cache.json";
    fn read() -> Option<Self> {
        match fs::read(CacheInfo::FILE) {
            Ok(contents) => match serde_json::from_slice(&contents) {
                Ok(cached) => Some(cached),
                Err(e) => {
                    eprintln!("Cache file {} could not be deserialized: {}", CacheInfo::FILE, e);
                    None
                }
            }
            Err(e) => {
                if e.kind() != NotFound {
                    eprintln!("Cache file {} could not be read: {}", CacheInfo::FILE, e);
                }
                None
            }
        }
    }
    fn save(build: &BuildInfo,  current: &BuildState) {
        let contents = serde_json::to_string_pretty(&CacheInfo {
            build: Cow::Borrowed(build),
            check: Cow::Borrowed(current),
        }).expect("serialization to be infallible");
        if let Err(e) = fs::write(CacheInfo::FILE, contents) {
            eprintln!("Failed to write cache file {}: {}", CacheInfo::FILE, e);
        } else {
            eprintln!("Cached compile command for future runs");
            eprintln!("\t(It is automatically invalidated by changes to Cargo.lock, rustc version or cargo args.)");
        }
    }
}

/// Returns a list of extra command args on success
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
fn extract_final_rustc(build_plan: &serde_json::Value) -> Option<BuildInfo> {
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
    let mut command = SerializableCommand::new(program);
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

    // If the program root is inside a subdirectory (like src/), watch the
    // entire directoy, otherwise, assume it's a single file and only watch it.
    // Use Path to split directories properly. The path is guaranteed to be
    // unicode because cargo fails otherwise.
    let src_root = match args.iter().find(|arg| arg.ends_with(".rs") ) {
        Some(main) => {
            let path = Path::new(main);
            let path = path.parent().filter(|&p| p != Path::new("") ).unwrap_or(path);
            path.to_str().unwrap().to_string()
        }
        None => {
            eprintln!("no .rs file in {} invocation, falling back to watching src/", program);
            "src".to_string()
        }
    };
    Some(BuildInfo { src_root, command, exe, link })
}

/// Fallback when getting or parsing a build plan fails
fn default_cargo(cargo_args: &[Box<str>]) -> BuildInfo {
    eprintln!("falling back to using `cargo build` and looking for changes in src/");
    let mut cargo = SerializableCommand::new("cargo");
    cargo.args(&["build"]);
    //cargo.args(&["--verbose"]);
    cargo.args(cargo_args);
    // see extract_final_rustc()
    cargo.env("RUSTFLAGS", "-C link-args=-Wl,-export-dynamic");
    BuildInfo {
        command: cargo,
        src_root: "src".to_string(),
        link: get_exe().unwrap(),
        exe: get_exe().unwrap(),
    }
}


/// Get command to compile the final executable assuming unchanged dependencies
/// Tries to extract a command from a nightly `cargo --build-plan`, but falls
/// back to the slower `cargo build` on errors.
/// Prints errors and warnings to stderr
///
/// Does not try to use rustup to get nightly, because
/// a) it might not be installed, and having two paths complicate the code.
/// b) would not allow people to use a specific nightly.
/// c) build plan has toolchain-specific --incremental arguments and .rlib IDs,
///    which means you need to build with the same (nightly) toolchain;
///    people need to build the initial binary as nightly as loading a library
///    compiled with different compiler is a bad idea.
///
/// FIXME: try without "-Zunstable-options", (but that complicates the code)
fn from_build_plan(cargo_args: &[Box<str>]) -> Result<BuildInfo,bool> {
    let output = Command::new("cargo")
        .args(&["build","-Zunstable-options","--build-plan"])
        .args(cargo_args.iter().map(|s| s.as_ref() ))
        .output();
    let stdout = match output {
        Err(e) => {
            eprintln!("Cannot run cargo: {}", &e);
            return Err(false);
        }
        Ok(ref output) if !output.status.success() => {
            eprintln!("`cargo --build-plan` failed (");
            let _ = io::stderr().write(&output.stderr);
            eprint!("), --build-plan requires a nightly toolchain, ");
            eprintln!("You might want to run `rustup override set nightly`.");
            return Err(true);
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
            return Err(true);
        }
    };
    if let Some(mut build_info) = extract_final_rustc(&json) {
        match run_custom_build_steps(&json) {
            Some(args) => {build_info.command.args(args);}
            None => eprintln!("Parsing custom build steps failed, trying without.")
        }
        Ok(build_info)
    } else {
        eprint!("Could not get rustc command from build plan, ");
        Err(true)
    }
}

/// Read from cache if possible, parse
fn get_compile_info(cargo_args: Vec<Box<str>>) -> Option<BuildInfo> {
    let current_bs = BuildState::current(&cargo_args);
    if let Some(CacheInfo { check: cached_bs,  build: build_info }) = CacheInfo::read() {
        if let Some(ref current_bs) = &current_bs {
            if &*cached_bs == current_bs {
                return Some(build_info.into_owned());
            }
            println!("Cannot use cached compile command due to changed state:");
            println!("cached: {:#?}", cached_bs);
            println!("now: {:#?}", current_bs);
        } else {
            println!("Cannot use cached compile command due to unknown state");
        }
    }
    let build_info = match from_build_plan(&cargo_args) {
        Ok(build_info) => build_info,
        Err(true) => return Some(default_cargo(&cargo_args)), // don't try to cache
        Err(false) => return None,
    };
    if let Some(build_state) = current_bs {
        CacheInfo::save(&build_info, &build_state);
    }
    Some(build_info)
}

/// Wrapper around std::env::current_exe() that removes junk, ensures UTF-8 and prints errors.
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


/// Load exe as a dynamic library, get the new function pointers
/// and do some trivial sanity checks.
///
/// This is kinda unsafe but the unsafety must end somewhere.
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
                // leak the handle because unloading is very risky.
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


/// Watch for changes in / to src and call a function per change per second
fn watch(src: &str,  callback: &mut dyn FnMut(PathBuf)) {
    'rewatch: loop {
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = match Watcher::new(tx, Duration::from_secs(1)) {
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

        'next: loop {
            match rx.recv() {
                Ok(DebouncedEvent::Write(path)) => callback(path),
                Ok(DebouncedEvent::Create(path)) => callback(path),
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
                Ok(e) => println!("other watch event: {:?}", e)
                //Ok(_) => {}
            }
        }
    }
}

pub struct FunctionGetter(Arc<AtomicPtr<Functions>>);
impl FunctionGetter {
    #[inline(never)]
    pub fn new(f: Functions,  cargo_args: Vec<Box<str>>) -> Self {
        let f = Arc::new(AtomicPtr::new(Box::leak(Box::new(f))));
        let f_clone = f.clone();
        thread::spawn(move|| {
            let functions = f_clone;
            // Don't delay game start on getting the build plan
            if let Some(b) = get_compile_info(cargo_args) {
                let BuildInfo { src_root, exe, link, command } = b;
                println!("Watching {:?} for source code changes", &src_root);
                let mut command = command.create();
                //command = default_cargo(&cargo_args).1;
                println!("command: {:?}", &command);
                watch(&src_root, &mut|_| {
                    // TODO add minimum delay between successful recompiles,
                    // maybe cancel running compiles (although that might corrupt
                    //  incremental builds?)
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
