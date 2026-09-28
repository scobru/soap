//! Windows diagnostics: load the Clear core's DLLs one at a time, in
//! dependency order, to find the one whose initialization blocks.
//! `cargo run --example probe_dlls -- <folder>`

#[cfg(windows)]
fn main() {
    use libloading::os::windows::{Library, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR};
    use std::io::Write;

    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("usage: probe_dlls <folder>"));
    let order = [
        "VCRUNTIME140.dll",
        "VCRUNTIME140_1.dll",
        "VCRUNTIME140_THREADS.dll",
        "MSVCP140.dll",
        "swiftCore.dll",
        "swiftCRT.dll",
        "swiftWinSDK.dll",
        "BlocksRuntime.dll",
        "dispatch.dll",
        "swiftDispatch.dll",
        "swift_Concurrency.dll",
        "swift_RegexParser.dll",
        "swift_StringProcessing.dll",
        "_FoundationICU.dll",
        "FoundationEssentials.dll",
        "FoundationInternationalization.dll",
        "Foundation.dll",
        "FoundationNetworking.dll",
        "libLiteRt.dll",
        "ClearNode.dll",
    ];
    for name in order {
        let path = dir.join(name);
        if !path.is_file() {
            eprintln!("skip     {name} (not bundled)");
            continue;
        }
        eprint!("loading  {name} ... ");
        std::io::stderr().flush().ok();
        let started = std::time::Instant::now();
        match unsafe { Library::load_with_flags(&path, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) } {
            Ok(lib) => {
                eprintln!("ok ({} ms)", started.elapsed().as_millis());
                std::mem::forget(lib);
            }
            Err(e) => eprintln!("FAILED: {e}"),
        }
    }
    eprintln!("all loaded");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("probe_dlls is Windows-only");
}
