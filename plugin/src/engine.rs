//! Bindings to the Clear native core (`libClearNode` / `ClearNode.dll`) built
//! from Desert Ant Labs' desert-ant-core. It exposes the generic `dal_*` C ABI;
//! options and results cross it as big-endian payloads (see the SDK's `codec.js`).

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::OnceLock;

type CreateFn = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_void;
type HandleIntFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type RunFn = unsafe extern "C" fn(
    *mut c_void,
    *const u8,
    c_int,
    *const u8,
    c_int,
    *const c_char,
    *const c_char,
) -> *mut c_void;
type PtrFn = unsafe extern "C" fn(*mut c_void);
type ProbeFn = unsafe extern "C" fn() -> c_int;

const MODEL_ID: &str = "clear";

#[cfg(target_os = "linux")]
const CORE_FILE: &str = "libClearNode.so";
#[cfg(target_os = "macos")]
const CORE_FILE: &str = "libClearNode.dylib";
#[cfg(target_os = "windows")]
const CORE_FILE: &str = "ClearNode.dll";
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const CORE_FILE: &str = "libClearNode.so";

/// `<platform>-<arch>` as the npm package names its `native/` folders.
pub fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "aarch64") => "darwin-arm64",
        ("windows", "x86_64") => "windows-x64",
        _ => "unsupported",
    }
}

pub fn default_native_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("soap-voice").join("native").join(platform_key()))
}

/// Print a loading step when `SOAP_DEBUG_LOAD` is set (diagnostics only).
fn trace(message: impl FnOnce() -> String) {
    if std::env::var_os("SOAP_DEBUG_LOAD").is_some() {
        eprintln!("[soap] {}", message());
    }
}

/// Where the Clear core may live, tried in order and only as far as needed:
/// `SOAP_NATIVE_DIR`, then beside the plugin binary (the bundle's
/// `Resources/native` on macOS, next to the `.clap` elsewhere), then the
/// per-user folder `install-native.sh` uses.
fn find_native_dir() -> Result<PathBuf, String> {
    let mut tried = Vec::new();
    let mut check = |dir: PathBuf| -> Option<PathBuf> {
        trace(|| format!("looking in {}", dir.display()));
        if dir.join(CORE_FILE).is_file() {
            Some(dir)
        } else {
            tried.push(dir.display().to_string());
            None
        }
    };
    if let Some(dir) = std::env::var_os("SOAP_NATIVE_DIR") {
        if let Some(found) = check(PathBuf::from(dir)) {
            return Ok(found);
        }
    }
    trace(|| "locating the plugin binary".into());
    if let Some(dir) = own_module_path().as_deref().and_then(Path::parent) {
        for candidate in [dir.join("..").join("Resources").join("native"), dir.join("native"), dir.to_path_buf()] {
            if let Some(found) = check(candidate) {
                return Ok(found);
            }
        }
    }
    trace(|| "locating the user data folder".into());
    if let Some(found) = default_native_dir().and_then(&mut check) {
        return Ok(found);
    }
    Err(format!(
        "Clear native library not found (looked in: {}). Reinstall the plugin package.",
        tried.join(", ")
    ))
}

/// Path of the binary this code lives in (the plugin, not the host).
#[cfg(unix)]
fn own_module_path() -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let found = unsafe { libc::dladdr(own_module_path as *const c_void, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(info.dli_fname) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes())))
}

#[cfg(windows)]
fn own_module_path() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };
    let mut module: HMODULE = std::ptr::null_mut();
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    if unsafe { GetModuleHandleExW(flags, own_module_path as *const u16, &mut module) } == 0 {
        return None;
    }
    let mut buf = vec![0u16; 32_768];
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    (len > 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
}

#[cfg(not(any(unix, windows)))]
fn own_module_path() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
unsafe fn load_library(path: &Path) -> Result<Library, libloading::Error> {
    use libloading::os::windows::{Library as WinLibrary, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR};
    use windows_sys::Win32::System::Diagnostics::Debug::{
        SetThreadErrorMode, SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX,
    };
    // A missing or mismatched dependency would otherwise raise a modal system
    // dialog that blocks the host; report it as an error instead.
    let mut previous = 0;
    trace(|| "SetThreadErrorMode".into());
    SetThreadErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX, &mut previous);
    trace(|| "LoadLibraryExW".into());
    // Resolve the core's own dependencies (LiteRT, ONNX Runtime, the Swift
    // runtime) from its folder first, never from PATH or System32 copies.
    let result =
        WinLibrary::load_with_flags(path, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS).map(Into::into);
    SetThreadErrorMode(previous, std::ptr::null_mut());
    result
}

#[cfg(not(windows))]
unsafe fn load_library(path: &Path) -> Result<Library, libloading::Error> {
    Library::new(path)
}

struct ClearLib {
    create: CreateFn,
    is_downloaded: HandleIntFn,
    download: HandleIntFn,
    run: RunFn,
    destroy: PtrFn,
    buffer_free: PtrFn,
    dir: PathBuf,
    // Held for the lifetime of the process (in a static): the Swift core must never be unloaded.
    _core: Library,
}

impl ClearLib {
    fn get() -> Result<&'static ClearLib, String> {
        static LIB: OnceLock<Result<ClearLib, String>> = OnceLock::new();
        let lib = LIB.get_or_init(Self::load);
        trace(|| "library initialised".into());
        lib.as_ref().map_err(Clone::clone)
    }

    fn load() -> Result<ClearLib, String> {
        if platform_key() == "unsupported" {
            return Err(format!(
                "Clear has no native build for {}-{}: supported are Windows x64, macOS Apple Silicon and Linux x64/arm64.",
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
        }
        let dir = find_native_dir()?;
        unsafe { Self::open(&dir) }
    }

    unsafe fn open(dir: &Path) -> Result<ClearLib, String> {
        let err = |e: libloading::Error| format!("Could not load the Clear library: {e}");
        // On Linux the core finds libLiteRt.so beside it through its $ORIGIN runpath.
        trace(|| format!("loading {}", dir.join(CORE_FILE).display()));
        // Never unloaded, not even on the error paths below: FreeLibrary on the
        // Swift runtime deadlocks on Windows.
        let core = std::mem::ManuallyDrop::new(load_library(&dir.join(CORE_FILE)).map_err(err)?);
        trace(|| "resolving symbols".into());

        if cfg!(target_os = "linux") {
            if let Ok(curl) = core.get::<ProbeFn>(b"dal_curl_available\0") {
                if curl() == 0 {
                    return Err("Clear needs libcurl (libcurl4 on Debian/Ubuntu) to download the model.".into());
                }
            }
        }

        let create_symbol = format!("{MODEL_ID}_create\0");
        let symbol = |name: &[u8]| {
            trace(|| format!("GetProcAddress {}", String::from_utf8_lossy(&name[..name.len() - 1])));
            core.get::<unsafe extern "C" fn()>(name).map(|f| *f).map_err(err)
        };
        let create = symbol(create_symbol.as_bytes())?;
        let is_downloaded = symbol(b"dal_is_downloaded\0")?;
        let download = symbol(b"dal_download\0")?;
        let run = symbol(b"dal_run\0")?;
        let destroy = symbol(b"dal_destroy\0")?;
        let buffer_free = symbol(b"dal_buffer_free\0")?;
        trace(|| "Clear core ready".into());
        Ok(ClearLib {
            create: std::mem::transmute::<unsafe extern "C" fn(), CreateFn>(create),
            is_downloaded: std::mem::transmute::<unsafe extern "C" fn(), HandleIntFn>(is_downloaded),
            download: std::mem::transmute::<unsafe extern "C" fn(), HandleIntFn>(download),
            run: std::mem::transmute::<unsafe extern "C" fn(), RunFn>(run),
            destroy: std::mem::transmute::<unsafe extern "C" fn(), PtrFn>(destroy),
            buffer_free: std::mem::transmute::<unsafe extern "C" fn(), PtrFn>(buffer_free),
            dir: dir.to_path_buf(),
            _core: std::mem::ManuallyDrop::into_inner(core),
        })
    }
}

/// Load the Clear core now (it is otherwise loaded on first use) and report
/// the folder it came from.
pub fn preload() -> Result<PathBuf, String> {
    ClearLib::get().map(|lib| lib.dir.clone())
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnhanceOptions {
    /// Blend between original (0) and fully enhanced (1).
    pub strength: f64,
    /// Integrated loudness target; `None` skips mastering.
    pub target_lufs: Option<f64>,
    pub peak_ceiling_dbfs: f64,
    pub max_gain_db: f64,
    pub output_sample_rate: f64,
    pub mono_downmix: bool,
}

#[derive(Clone, Debug)]
pub struct EnhanceResult {
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: f64,
    pub duration_sec: f64,
    pub processing_sec: f64,
    pub measured_lufs: Option<f64>,
    pub measured_true_peak_dbfs: Option<f64>,
}

impl EnhanceResult {
    pub fn realtime_factor(&self) -> f64 {
        if self.processing_sec > 0.0 {
            self.duration_sec / self.processing_sec
        } else {
            0.0
        }
    }
}

pub struct ClearModel {
    lib: &'static ClearLib,
    handle: NonNull<c_void>,
}

// The handle is only ever used from one thread at a time (the worker).
unsafe impl Send for ClearModel {}

impl ClearModel {
    /// Bind the native core and create a model handle, using the SDK's managed
    /// cache (`~/.cache/desert-ant-models` on Linux).
    pub fn open() -> Result<Self, String> {
        let lib = ClearLib::get()?;
        let id = CString::new(MODEL_ID).unwrap();
        let handle = unsafe { (lib.create)(id.as_ptr(), std::ptr::null(), std::ptr::null()) };
        NonNull::new(handle)
            .map(|handle| Self { lib, handle })
            .ok_or_else(|| "The Clear core could not create the model.".to_string())
    }

    pub fn is_downloaded(&self) -> bool {
        unsafe { (self.lib.is_downloaded)(self.handle.as_ptr()) != 0 }
    }

    /// Download the pinned weights from Hugging Face. Blocking.
    pub fn download(&self) -> Result<(), String> {
        match unsafe { (self.lib.download)(self.handle.as_ptr()) } {
            0 => Ok(()),
            _ => Err("Could not download the Clear model from Hugging Face. Check your connection.".into()),
        }
    }

    /// Denoise, dereverb and (optionally) master `channels`. Blocking.
    pub fn enhance(
        &self,
        channels: &[Vec<f32>],
        sample_rate: f64,
        options: &EnhanceOptions,
    ) -> Result<EnhanceResult, String> {
        let input = encode_input(channels, sample_rate);
        let opts = encode_options(options);
        let (Ok(input_len), Ok(opts_len)) = (c_int::try_from(input.len()), c_int::try_from(opts.len())) else {
            return Err("Capture too long to process in one pass.".into());
        };
        let ptr = unsafe {
            (self.lib.run)(
                self.handle.as_ptr(),
                input.as_ptr(),
                input_len,
                opts.as_ptr(),
                opts_len,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if ptr.is_null() {
            return Err("The Clear model failed to process the audio.".into());
        }
        let result = unsafe {
            let head = std::slice::from_raw_parts(ptr as *const u8, 4);
            let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
            let payload = std::slice::from_raw_parts((ptr as *const u8).add(4), len);
            decode_result(payload)
        };
        unsafe { (self.lib.buffer_free)(ptr) };
        result
    }
}

impl Drop for ClearModel {
    fn drop(&mut self) {
        unsafe { (self.lib.destroy)(self.handle.as_ptr()) };
    }
}

fn push_f32_array(out: &mut Vec<u8>, values: &[f32]) {
    out.extend_from_slice(&(values.len() as u32).to_be_bytes());
    out.reserve(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

pub(crate) fn encode_input(channels: &[Vec<f32>], sample_rate: f64) -> Vec<u8> {
    let mut out = Vec::new();
    push_f32_array(&mut out, &channels[0]);
    out.extend_from_slice(&sample_rate.to_be_bytes());
    out.extend_from_slice(&((channels.len() - 1) as u32).to_be_bytes());
    for ch in &channels[1..] {
        push_f32_array(&mut out, ch);
    }
    out
}

pub(crate) fn encode_options(o: &EnhanceOptions) -> Vec<u8> {
    [
        o.strength,
        o.target_lufs.unwrap_or(f64::NAN),
        o.peak_ceiling_dbfs,
        o.max_gain_db,
        o.output_sample_rate,
        if o.mono_downmix { 1.0 } else { 0.0 },
        f64::NAN, // balanceChannelsLUFS: leave the stereo balance alone
    ]
    .iter()
    .flat_map(|v| v.to_be_bytes())
    .collect()
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }

    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        if self.remaining() < n {
            return Err("Truncated result from the Clear core.".into());
        }
        let s = &self.bytes[self.at..self.at + n];
        self.at += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn f32_array(&mut self) -> Result<Vec<f32>, String> {
        let n = self.u32()? as usize;
        Ok(self
            .take(n * 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_be_bytes(*b))
            .collect())
    }
}

pub(crate) fn decode_result(payload: &[u8]) -> Result<EnhanceResult, String> {
    let mut r = Reader { bytes: payload, at: 0 };
    let first = r.f32_array()?;
    let sample_rate = r.f64()?;
    let duration_sec = r.f64()?;
    let processing_sec = r.f64()?;
    let lufs = r.f64()?;
    let peak = if r.remaining() >= 8 { r.f64()? } else { f64::NAN };
    let mut channels = vec![first];
    if r.remaining() >= 4 {
        for _ in 0..r.u32()? {
            channels.push(r.f32_array()?);
        }
    }
    let finite = |v: f64| (!v.is_nan()).then_some(v);
    Ok(EnhanceResult {
        channels,
        sample_rate,
        duration_sec,
        processing_sec,
        measured_lufs: finite(lufs),
        measured_true_peak_dbfs: finite(peak),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_payload_matches_the_js_codec() {
        let bytes = encode_options(&EnhanceOptions {
            strength: 0.7,
            target_lufs: None,
            peak_ceiling_dbfs: -1.5,
            max_gain_db: 9.0,
            output_sample_rate: 44_100.0,
            mono_downmix: false,
        });
        let values: Vec<f64> = bytes.as_chunks::<8>().0.iter().map(|b| f64::from_be_bytes(*b)).collect();
        assert_eq!(values.len(), 7);
        assert_eq!(values[0], 0.7);
        assert!(values[1].is_nan());
        assert_eq!(&values[2..6], &[-1.5, 9.0, 44_100.0, 0.0]);
        assert!(values[6].is_nan());
    }

    #[test]
    fn input_payload_layout() {
        let bytes = encode_input(&[vec![0.5, -0.25], vec![1.0, 0.0]], 48_000.0);
        assert_eq!(&bytes[0..4], &2u32.to_be_bytes());
        assert_eq!(&bytes[4..8], &0.5f32.to_be_bytes());
        assert_eq!(&bytes[12..20], &48_000f64.to_be_bytes());
        assert_eq!(&bytes[20..24], &1u32.to_be_bytes());
        assert_eq!(bytes.len(), 4 + 8 + 8 + 4 + 4 + 8);
    }

    #[test]
    fn decodes_a_stereo_result() {
        let mut p = Vec::new();
        push_f32_array(&mut p, &[0.1, 0.2]);
        for v in [48_000.0, 2.0 / 48_000.0, 0.001, -20.5, -1.6] {
            p.extend_from_slice(&f64::to_be_bytes(v));
        }
        p.extend_from_slice(&1u32.to_be_bytes());
        push_f32_array(&mut p, &[0.3, 0.4]);
        let r = decode_result(&p).unwrap();
        assert_eq!(r.channels, vec![vec![0.1, 0.2], vec![0.3, 0.4]]);
        assert_eq!(r.measured_lufs, Some(-20.5));
        assert_eq!(r.measured_true_peak_dbfs, Some(-1.6));
    }

    #[test]
    fn decodes_bypassed_mastering_as_none() {
        let mut p = Vec::new();
        push_f32_array(&mut p, &[0.0]);
        for v in [48_000.0, 1.0, 1.0, f64::NAN, f64::NAN] {
            p.extend_from_slice(&f64::to_be_bytes(v));
        }
        let r = decode_result(&p).unwrap();
        assert_eq!(r.measured_lufs, None);
        assert_eq!(r.channels.len(), 1);
    }
}
