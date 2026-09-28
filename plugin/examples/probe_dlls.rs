//! Windows diagnostics for loading the Clear core. Each mode runs in its own
//! process (CI gives each a timeout), to isolate what makes loading block:
//!   individual     load every bundled DLL one by one, in dependency order
//!   direct         LoadLibrary ClearNode.dll alone, on the main thread
//!   direct-thread  the same on a spawned thread
//!   engine         soap::engine::preload() on the main thread
//!   engine-thread  preload() on a spawned thread
//!   engine-busy    preload() on a spawned thread while another thread sleeps
//!   sym            LoadLibrary, then GetProcAddress for each C ABI symbol
//!   sleep-sym      the same with a 5 s pause between them
//!   sleep-exit     LoadLibrary, pause, exit (does exit block?)
//!   litert-first   load libLiteRt.dll alone first, then as sleep-sym
//!   fallback       preload() with the plugin-path and user-folder lookups
//!   create         preload(), then clear_create and dal_is_downloaded
//! `cargo run --example probe_dlls -- <mode> <folder>`

#[cfg(windows)]
fn main() {
    use libloading::os::windows::{Library, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR};
    use std::io::Write;
    use std::path::{Path, PathBuf};

    fn load(path: &Path) {
        eprint!("loading  {} ... ", path.file_name().unwrap().to_string_lossy());
        std::io::stderr().flush().ok();
        let started = std::time::Instant::now();
        match unsafe { Library::load_with_flags(path, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) } {
            Ok(lib) => {
                eprintln!("ok ({} ms)", started.elapsed().as_millis());
                std::mem::forget(lib);
            }
            Err(e) => eprintln!("FAILED: {e}"),
        }
    }

    fn preload() {
        eprintln!("engine::preload() ...");
        let started = std::time::Instant::now();
        match soap::engine::preload() {
            Ok(dir) => eprintln!("preload ok ({} ms) from {}", started.elapsed().as_millis(), dir.display()),
            Err(e) => eprintln!("preload FAILED: {e}"),
        }
    }

    fn thread_count() -> usize {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        };
        let pid = std::process::id();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return 0;
            }
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut count = 0;
            let mut more = Thread32First(snapshot, &mut entry) != 0;
            while more {
                if entry.th32OwnerProcessID == pid {
                    count += 1;
                }
                more = Thread32Next(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            count
        }
    }

    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("usage: probe_dlls <mode> <folder>");
    let dir = PathBuf::from(args.next().expect("usage: probe_dlls <mode> <folder>"));
    std::env::set_var("SOAP_NATIVE_DIR", &dir);
    let core = dir.join("ClearNode.dll");

    match mode.as_str() {
        "individual" => {
            for name in [
                "VCRUNTIME140.dll", "VCRUNTIME140_1.dll", "VCRUNTIME140_THREADS.dll", "MSVCP140.dll",
                "swiftCore.dll", "swiftCRT.dll", "swiftWinSDK.dll", "BlocksRuntime.dll", "dispatch.dll",
                "swiftDispatch.dll", "swift_Concurrency.dll", "swift_RegexParser.dll",
                "swift_StringProcessing.dll", "_FoundationICU.dll", "FoundationEssentials.dll",
                "FoundationInternationalization.dll", "Foundation.dll", "FoundationNetworking.dll",
                "libLiteRt.dll", "ClearNode.dll",
            ] {
                let path = dir.join(name);
                if path.is_file() {
                    load(&path);
                }
            }
        }
        "direct" => load(&core),
        "sym" | "sleep-sym" | "sleep-exit" | "litert-first" => {
            let flags = LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS;
            eprintln!("threads before load: {}", thread_count());
            if mode == "litert-first" {
                let lrt = unsafe { Library::load_with_flags(dir.join("libLiteRt.dll"), flags) }.unwrap();
                std::mem::forget(lrt);
                eprintln!("libLiteRt.dll loaded; threads: {}", thread_count());
                std::thread::sleep(std::time::Duration::from_secs(3));
                eprintln!("after 3 s; threads: {}", thread_count());
            }
            let lib = unsafe { Library::load_with_flags(&core, flags) }.unwrap();
            eprintln!("ClearNode.dll loaded; threads: {}", thread_count());
            if mode != "sym" {
                std::thread::sleep(std::time::Duration::from_secs(5));
                eprintln!("after 5 s; threads: {}", thread_count());
            }
            if mode == "sleep-exit" {
                eprintln!("exiting");
                std::process::exit(0);
            }
            for name in ["clear_create", "dal_is_downloaded", "dal_download", "dal_run", "dal_destroy", "dal_buffer_free"] {
                eprint!("GetProcAddress({name}) ... ");
                std::io::stderr().flush().ok();
                let symbol = unsafe { lib.get::<unsafe extern "C" fn()>(format!("{name}\0").as_bytes()) };
                eprintln!("{}", if symbol.is_ok() { "ok" } else { "missing" });
            }
            std::mem::forget(lib);
        }
        "direct-thread" => std::thread::spawn(move || load(&core)).join().unwrap(),
        "engine" => preload(),
        "engine-thread" => std::thread::spawn(preload).join().unwrap(),
        "engine-busy" => {
            std::thread::spawn(|| std::thread::sleep(std::time::Duration::from_secs(600)));
            std::thread::spawn(preload).join().unwrap();
        }
        "fallback" => {
            // Skip SOAP_NATIVE_DIR so the plugin-path and user-folder lookups run.
            std::env::set_var("SOAP_NATIVE_DIR", dir.join("does-not-exist"));
            preload();
        }
        "create" => {
            preload();
            eprintln!("clear_create ...");
            let model = soap::engine::ClearModel::open().expect("clear_create");
            eprintln!("dal_is_downloaded ...");
            eprintln!("is_downloaded = {}", model.is_downloaded());
        }
        other => panic!("unknown mode {other}"),
    }
    eprintln!("done");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("probe_dlls is Windows-only");
}
