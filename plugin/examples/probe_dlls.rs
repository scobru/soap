//! Windows diagnostics for loading the Clear core. Each mode runs in its own
//! process (CI gives each a timeout), to isolate what makes loading block:
//!   individual     load every bundled DLL one by one, in dependency order
//!   direct         LoadLibrary ClearNode.dll alone, on the main thread
//!   direct-thread  the same on a spawned thread
//!   engine         soap::engine::preload() on the main thread
//!   engine-thread  preload() on a spawned thread
//!   engine-busy    preload() on a spawned thread while another thread sleeps
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
