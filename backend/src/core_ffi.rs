//! C++ Core 的安全封装（声明见 core/include/starraft/core.h）

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;

use serde::{Deserialize, Serialize};

extern "C" {
    fn sr_core_version() -> *const c_char;
    fn sr_free(ptr: *mut c_char);
    fn sr_os_info() -> *mut c_char;
    fn sr_detect_java() -> *mut c_char;
    #[allow(dead_code)]
    fn sr_java_version(java_path: *const c_char) -> *mut c_char;
    fn sr_sha1_file(path: *const c_char, out_hex: *mut c_char) -> c_int;
    fn sr_system_memory(total: *mut u64, available: *mut u64) -> c_int;
    fn sr_recommended_memory_mb() -> u32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstall {
    pub path: String,
    pub version: String,
    pub major: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OsInfo {
    pub os: String,
    pub arch: String,
}

/// 取得 Core 分配的字符串并释放
fn take(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: 指针来自 Core，保证以 0 结尾；读取后立刻交还 sr_free
    unsafe {
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        sr_free(ptr);
        Some(s)
    }
}

pub fn version() -> String {
    // SAFETY: 返回静态字符串
    unsafe { CStr::from_ptr(sr_core_version()).to_string_lossy().into_owned() }
}

pub fn os_info() -> OsInfo {
    take(unsafe { sr_os_info() }).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// 会启动若干 `java -version` 子进程，在异步上下文中请放进 spawn_blocking
pub fn detect_java() -> Vec<JavaInstall> {
    take(unsafe { sr_detect_java() }).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[allow(dead_code)]
pub fn java_version(path: &str) -> Option<String> {
    let c = CString::new(path).ok()?;
    take(unsafe { sr_java_version(c.as_ptr()) })
}

#[allow(dead_code)]
pub fn sha1_file(path: &Path) -> Option<String> {
    let c = CString::new(path.to_string_lossy().as_bytes()).ok()?;
    let mut buf = [0 as c_char; 41];
    let rc = unsafe { sr_sha1_file(c.as_ptr(), buf.as_mut_ptr()) };
    (rc == 0).then(|| unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
}

/// (总内存, 可用内存)，单位字节
pub fn memory() -> (u64, u64) {
    let (mut total, mut avail) = (0u64, 0u64);
    unsafe { sr_system_memory(&mut total, &mut avail) };
    (total, avail)
}

pub fn recommended_memory_mb() -> u32 {
    unsafe { sr_recommended_memory_mb() }
}
