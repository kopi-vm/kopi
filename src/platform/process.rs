// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Platform-specific process execution.

use crate::error::{KopiError, Result};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "macos")]
use std::collections::BTreeSet;
#[cfg(windows)]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(target_os = "macos")]
use libc::{c_char, gid_t, off_t, uid_t};
#[cfg(target_os = "macos")]
use libproc::libproc::bsd_info::BSDInfo;
#[cfg(target_os = "macos")]
use libproc::libproc::file_info::{ListFDs, PIDFDInfo, PIDFDInfoFlavor, ProcFDType, pidfdinfo};
#[cfg(target_os = "macos")]
use libproc::libproc::proc_pid::{listpidinfo, pidinfo, pidpath};
#[cfg(target_os = "macos")]
use libproc::processes::{ProcFilter, pids_by_type};
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStringExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use std::slice;
#[cfg(windows)]
use winapi::ctypes::c_void;
#[cfg(windows)]
use winapi::shared::minwindef::{DWORD, FALSE};
#[cfg(windows)]
use winapi::shared::ntdef::NTSTATUS;
#[cfg(windows)]
use winapi::shared::ntstatus::STATUS_INFO_LENGTH_MISMATCH;
#[cfg(windows)]
use winapi::shared::winerror::ERROR_INSUFFICIENT_BUFFER;
#[cfg(windows)]
use winapi::um::errhandlingapi::GetLastError;
#[cfg(windows)]
use winapi::um::fileapi::{GetFileType, GetFinalPathNameByHandleW};
#[cfg(windows)]
use winapi::um::handleapi::{CloseHandle, DuplicateHandle, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcess, TerminateProcess};
#[cfg(windows)]
use winapi::um::synchapi::WaitForSingleObject;
#[cfg(windows)]
use winapi::um::winbase::{
    FILE_TYPE_DISK, QueryFullProcessImageNameW, VOLUME_NAME_DOS, WAIT_OBJECT_0,
};
#[cfg(windows)]
use winapi::um::winnt::{
    DUPLICATE_SAME_ACCESS, HANDLE, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, SYNCHRONIZE,
};

#[cfg(windows)]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: u32,
        system_information: *mut c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> NTSTATUS;
}

/// Metadata about a process that is interacting with a JDK installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessInfo {
    /// Operating system process identifier.
    pub pid: u32,
    /// Path to the executable that owns the process.
    pub exe_path: PathBuf,
    /// Collection of open handle paths rooted inside the monitored JDK directory.
    pub handles: Vec<PathBuf>,
}

/// Enumerate processes that hold open handles beneath the provided directory.
pub fn processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>> {
    let canonical_target = normalize_target(target)?;
    platform_processes_using_path(&canonical_target)
}

/// Attempt to terminate a process by PID.
pub fn terminate_process(pid: u32) -> Result<()> {
    platform_terminate_process(pid)
}

#[cfg(windows)]
fn platform_terminate_process(pid: u32) -> Result<()> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, FALSE, pid);
        if handle.is_null() {
            return Err(KopiError::SystemError(format!(
                "Failed to open process {pid}: {}",
                std::io::Error::last_os_error()
            )));
        }

        if TerminateProcess(handle, 1) == 0 {
            let err = std::io::Error::last_os_error();
            CloseHandle(handle);
            return Err(KopiError::SystemError(format!(
                "Failed to terminate process {pid}: {err}"
            )));
        }

        let wait_result = WaitForSingleObject(handle, 5_000);
        if wait_result != WAIT_OBJECT_0 {
            CloseHandle(handle);
            return Err(KopiError::SystemError(format!(
                "Timed out waiting for process {pid} to exit (wait result {wait_result})"
            )));
        }

        if CloseHandle(handle) == 0 {
            return Err(KopiError::SystemError(format!(
                "Failed to close process handle for {pid}: {}",
                std::io::Error::last_os_error()
            )));
        }
    }

    Ok(())
}

#[cfg(unix)]
fn platform_terminate_process(pid: u32) -> Result<()> {
    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if result == 0 {
        Ok(())
    } else {
        Err(KopiError::SystemError(format!(
            "Failed to terminate process {pid}: {}",
            std::io::Error::last_os_error()
        )))
    }
}

#[cfg(not(any(unix, windows)))]
fn platform_terminate_process(_pid: u32) -> Result<()> {
    Err(KopiError::SystemError(
        "Process termination is not supported on this platform".to_string(),
    ))
}

fn normalize_target(target: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(target).map_err(|err| match err.kind() {
        ErrorKind::NotFound => KopiError::DirectoryNotFound(target.display().to_string()),
        ErrorKind::PermissionDenied => {
            KopiError::PermissionDenied(format!("Unable to access {}: {err}", target.display()))
        }
        _ => KopiError::SystemError(format!(
            "Failed to canonicalize {}: {err}",
            target.display()
        )),
    })?;

    let metadata = fs::metadata(&canonical).map_err(|err| match err.kind() {
        ErrorKind::PermissionDenied => {
            KopiError::PermissionDenied(format!("Unable to inspect {}: {err}", canonical.display()))
        }
        _ => KopiError::SystemError(format!("Failed to inspect {}: {err}", canonical.display())),
    })?;

    if !metadata.is_dir() {
        return Err(KopiError::ValidationError(format!(
            "Process detection target must be a directory: {}",
            canonical.display()
        )));
    }

    Ok(canonical)
}

#[cfg(target_os = "linux")]
fn platform_processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>> {
    linux_processes_using_path_with_root(Path::new("/proc"), target)
}

#[cfg(target_os = "linux")]
fn linux_processes_using_path_with_root(
    proc_root: &Path,
    target: &Path,
) -> Result<Vec<ProcessInfo>> {
    use std::collections::{BTreeMap, BTreeSet};

    let entries = fs::read_dir(proc_root).map_err(|err| {
        KopiError::SystemError(format!("Failed to read {}: {err}", proc_root.display()))
    })?;

    let mut processes: BTreeMap<u32, ProcessInfo> = BTreeMap::new();

    for entry_result in entries {
        let entry = match entry_result {
            Ok(value) => value,
            Err(err) => {
                log::debug!("skipping {proc_root:?} entry due to error: {err}");
                continue;
            }
        };

        let pid = match entry.file_name().to_string_lossy().parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        let proc_path = entry.path();
        let exe_link = proc_path.join("exe");
        let exe_path =
            fs::read_link(exe_link).unwrap_or_else(|_| PathBuf::from(format!("/proc/{pid}/exe")));

        let fd_dir = proc_path.join("fd");
        let fd_entries = match fs::read_dir(&fd_dir) {
            Ok(iter) => iter,
            Err(err) => {
                log::debug!("skipping fd inspection for pid {pid} due to error: {err}");
                continue;
            }
        };

        let mut handles = BTreeSet::new();

        for fd_entry in fd_entries {
            let fd_entry = match fd_entry {
                Ok(value) => value,
                Err(err) => {
                    log::debug!("skipping fd entry for pid {pid} due to error: {err}");
                    continue;
                }
            };

            let link_path = match fs::read_link(fd_entry.path()) {
                Ok(path) => path,
                Err(err) => {
                    log::debug!("unable to resolve fd symlink for pid {pid}: {err}");
                    continue;
                }
            };

            if !link_path.is_absolute() {
                continue;
            }

            let canonical_handle = match fs::canonicalize(&link_path) {
                Ok(path) => path,
                Err(err) => {
                    log::debug!("canonicalize failed for fd owned by pid {pid}: {err}");
                    continue;
                }
            };

            if canonical_handle.starts_with(target) {
                handles.insert(canonical_handle);
            }
        }

        if !handles.is_empty() {
            processes.insert(
                pid,
                ProcessInfo {
                    pid,
                    exe_path: exe_path.clone(),
                    handles: handles.into_iter().collect(),
                },
            );
        }
    }

    Ok(processes.into_values().collect())
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RawFsid {
    val: [i32; 2],
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RawVinfoStat {
    vst_dev: u32,
    vst_mode: u16,
    vst_nlink: u16,
    vst_ino: u64,
    vst_uid: uid_t,
    vst_gid: gid_t,
    vst_atime: i64,
    vst_atimensec: i64,
    vst_mtime: i64,
    vst_mtimensec: i64,
    vst_ctime: i64,
    vst_ctimensec: i64,
    vst_birthtime: i64,
    vst_birthtimensec: i64,
    vst_size: off_t,
    vst_blocks: i64,
    vst_blksize: i32,
    vst_flags: u32,
    vst_gen: u32,
    vst_rdev: u32,
    vst_qspare: [i64; 2],
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RawVnodeInfo {
    vi_stat: RawVinfoStat,
    vi_type: i32,
    vi_pad: i32,
    vi_fsid: RawFsid,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RawProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: off_t,
    fi_type: i32,
    fi_guardflags: u32,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
#[repr(C)]
struct RawVnodeInfoPath {
    vip_vi: RawVnodeInfo,
    vip_path: [c_char; 1024],
}

#[cfg(target_os = "macos")]
impl Default for RawVnodeInfoPath {
    fn default() -> Self {
        Self {
            vip_vi: RawVnodeInfo::default(),
            vip_path: [0 as c_char; 1024],
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RawVnodeFDInfoWithPath {
    pfi: RawProcFileInfo,
    pvip: RawVnodeInfoPath,
}

#[cfg(target_os = "macos")]
impl PIDFDInfo for RawVnodeFDInfoWithPath {
    fn flavor() -> PIDFDInfoFlavor {
        PIDFDInfoFlavor::VNodePathInfo
    }
}

#[cfg(target_os = "macos")]
fn extract_vnode_path(buffer: &[c_char; 1024]) -> Option<PathBuf> {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();

    if bytes.is_empty() {
        return None;
    }

    let os_string = OsString::from_vec(bytes);
    if os_string.is_empty() {
        None
    } else {
        Some(PathBuf::from(os_string))
    }
}

#[cfg(target_os = "macos")]
fn record_handle_if_within_target(
    target: &Path,
    pid: u32,
    path: &Path,
    handles: &mut BTreeSet<PathBuf>,
) {
    match fs::canonicalize(path) {
        Ok(canonical) => {
            if canonical.starts_with(target) {
                handles.insert(canonical);
            }
        }
        Err(err) => {
            if path.starts_with(target) {
                handles.insert(path.to_path_buf());
            } else {
                log::debug!("skipping handle for pid {pid} at {}: {err}", path.display());
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>> {
    let mut processes: Vec<ProcessInfo> = Vec::new();

    let pids = pids_by_type(ProcFilter::All)
        .map_err(|err| KopiError::SystemError(format!("Failed to enumerate processes: {err}")))?;

    for pid in pids {
        if pid == 0 {
            continue;
        }

        let pid_i32 = pid as i32;

        let exe_path = match pidpath(pid_i32) {
            Ok(path) => PathBuf::from(path),
            Err(err) => {
                log::debug!("skipping pid {pid} due to pidpath failure: {err}");
                continue;
            }
        };

        let bsd_info = match pidinfo::<BSDInfo>(pid_i32, 0) {
            Ok(info) => info,
            Err(err) => {
                log::debug!("skipping pid {pid} due to pidinfo failure: {err}");
                continue;
            }
        };

        let descriptor_capacity = bsd_info.pbi_nfiles as usize;
        if descriptor_capacity == 0 {
            continue;
        }

        let descriptors = match listpidinfo::<ListFDs>(pid_i32, descriptor_capacity) {
            Ok(list) => list,
            Err(err) => {
                log::debug!("skipping fd enumeration for pid {pid}: {err}");
                continue;
            }
        };

        if descriptors.is_empty() {
            continue;
        }

        let mut handles = BTreeSet::new();

        for descriptor in descriptors {
            if !matches!(ProcFDType::from(descriptor.proc_fdtype), ProcFDType::VNode) {
                continue;
            }

            let vnode_info = match pidfdinfo::<RawVnodeFDInfoWithPath>(pid_i32, descriptor.proc_fd)
            {
                Ok(info) => info,
                Err(err) => {
                    log::debug!(
                        "pidfdinfo failed for pid {pid} fd {}: {err}",
                        descriptor.proc_fd
                    );
                    continue;
                }
            };

            if let Some(handle_path) = extract_vnode_path(&vnode_info.pvip.vip_path) {
                record_handle_if_within_target(target, pid, &handle_path, &mut handles);
            }
        }

        if !handles.is_empty() {
            processes.push(ProcessInfo {
                pid,
                exe_path,
                handles: handles.into_iter().collect(),
            });
        }
    }

    processes.sort_by_key(|info| info.pid);
    Ok(processes)
}

#[cfg(windows)]
fn platform_processes_using_path(target: &Path) -> Result<Vec<ProcessInfo>> {
    const INITIAL_BUFFER_SIZE: usize = 1 << 20;
    const SYSTEM_EXTENDED_HANDLE_INFORMATION: u32 = 64;

    let mut buffer_size = INITIAL_BUFFER_SIZE;
    let mut buffer = vec![0u8; buffer_size];
    let mut return_length: u32 = 0;

    loop {
        let status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_EXTENDED_HANDLE_INFORMATION,
                buffer.as_mut_ptr() as *mut c_void,
                buffer_size as u32,
                &mut return_length,
            )
        };

        if status == STATUS_INFO_LENGTH_MISMATCH {
            let required = return_length as usize;
            buffer_size = required
                .checked_add(required / 2)
                .unwrap_or(buffer_size.saturating_mul(2))
                .max(buffer_size.saturating_mul(2));
            buffer.resize(buffer_size, 0);
            continue;
        }

        if !nt_success(status) {
            return Err(KopiError::SystemError(format!(
                "NtQuerySystemInformation failed with status 0x{status:08X}",
                status = status as u32
            )));
        }

        break;
    }

    let info_ptr = buffer.as_ptr() as *const SystemHandleInformationEx;
    let handle_info = unsafe { &*info_ptr };
    let handle_count = handle_info.NumberOfHandles as usize;
    let handle_slice = unsafe { slice::from_raw_parts(handle_info.Handles.as_ptr(), handle_count) };

    let mut collector = WindowsProcessCollector::new(target);
    let mut exe_cache: BTreeMap<u32, PathBuf> = BTreeMap::new();
    let mut skipped_pids: BTreeSet<u32> = BTreeSet::new();
    let current_process = unsafe { GetCurrentProcess() };

    for entry in handle_slice {
        let pid = entry.UniqueProcessId as u32;
        if pid == 0 || skipped_pids.contains(&pid) {
            continue;
        }

        let Some(process_handle) = open_process(pid) else {
            skipped_pids.insert(pid);
            continue;
        };

        let exe_path = match exe_cache.get(&pid) {
            Some(cached) => cached.clone(),
            None => match query_process_executable(process_handle.raw()) {
                Some(path) => {
                    exe_cache.insert(pid, path.clone());
                    path
                }
                None => {
                    log::debug!("unable to resolve executable path for pid {pid}");
                    skipped_pids.insert(pid);
                    continue;
                }
            },
        };

        let Some(duplicated) =
            duplicate_handle_into_current(process_handle.raw(), entry.HandleValue, current_process)
        else {
            continue;
        };

        if !is_disk_file(&duplicated) {
            continue;
        }

        let Some(handle_path) = get_handle_path(&duplicated) else {
            continue;
        };

        collector.add_handle(pid, &exe_path, handle_path);
    }

    let mut processes = collector.finish();
    processes.sort_by_key(|info| info.pid);
    Ok(processes)
}

#[cfg(windows)]
fn nt_success(status: NTSTATUS) -> bool {
    status >= 0
}

#[cfg(windows)]
const FILE_NAME_NORMALIZED: DWORD = 0;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_processes_using_path(_target: &Path) -> Result<Vec<ProcessInfo>> {
    Err(KopiError::NotImplemented(
        "Process activity detection is not supported on this platform".to_string(),
    ))
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct SystemHandleInformationEx {
    NumberOfHandles: usize,
    Reserved: usize,
    Handles: [SystemHandleTableEntryInfoEx; 1],
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct SystemHandleTableEntryInfoEx {
    Object: *mut c_void,
    UniqueProcessId: usize,
    HandleValue: usize,
    GrantedAccess: u32,
    CreatorBackTraceIndex: u16,
    ObjectTypeIndex: u16,
    HandleAttributes: u32,
    Reserved: u32,
}

#[cfg(windows)]
struct HandleGuard(HANDLE);

#[cfg(windows)]
impl HandleGuard {
    fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

#[cfg(windows)]
impl Drop for HandleGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn open_process(pid: u32) -> Option<HandleGuard> {
    let desired = PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION;
    let handle = unsafe { OpenProcess(desired, FALSE, pid) };

    match HandleGuard::new(handle) {
        Some(guard) => Some(guard),
        None => {
            let error = unsafe { GetLastError() };
            log::debug!("OpenProcess failed for pid {pid} (error 0x{error:08X})");
            None
        }
    }
}

#[cfg(windows)]
fn duplicate_handle_into_current(
    source_process: HANDLE,
    handle_value: usize,
    current_process: HANDLE,
) -> Option<HandleGuard> {
    let mut duplicated: HANDLE = ptr::null_mut();
    let source_handle = handle_value as *mut c_void;
    let success = unsafe {
        DuplicateHandle(
            source_process,
            source_handle,
            current_process,
            &mut duplicated,
            0,
            FALSE,
            DUPLICATE_SAME_ACCESS,
        )
    };

    if success == FALSE {
        let error = unsafe { GetLastError() };
        log::debug!("DuplicateHandle failed (error 0x{error:08X})");
        return None;
    }

    HandleGuard::new(duplicated)
}

#[cfg(windows)]
fn is_disk_file(handle: &HandleGuard) -> bool {
    unsafe { GetFileType(handle.raw()) == FILE_TYPE_DISK }
}

#[cfg(windows)]
fn query_process_executable(process: HANDLE) -> Option<PathBuf> {
    let mut capacity: u32 = 260;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let mut length = capacity;
        let success =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };

        if success != 0 {
            buffer.truncate(length as usize);
            let os_string = OsString::from_wide(&buffer);
            return Some(PathBuf::from(os_string));
        }

        let error = unsafe { GetLastError() };
        if error == ERROR_INSUFFICIENT_BUFFER {
            capacity = capacity.saturating_mul(2);
            continue;
        }

        log::debug!("QueryFullProcessImageNameW failed with error {error}");
        return None;
    }
}

#[cfg(windows)]
fn get_handle_path(handle: &HandleGuard) -> Option<PathBuf> {
    let mut capacity: u32 = 512;
    let flags: DWORD = FILE_NAME_NORMALIZED | VOLUME_NAME_DOS;
    loop {
        let mut buffer = vec![0u16; capacity as usize];
        let length = unsafe {
            GetFinalPathNameByHandleW(handle.raw(), buffer.as_mut_ptr(), capacity, flags)
        };

        if length == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_INSUFFICIENT_BUFFER {
                capacity = capacity.saturating_mul(2);
                continue;
            }

            log::debug!("GetFinalPathNameByHandleW failed with error {error}");
            return None;
        }

        if length >= capacity {
            capacity = length + 1;
            continue;
        }

        buffer.truncate(length as usize);
        let os_string = OsString::from_wide(&buffer);
        let display = os_string.to_string_lossy();
        return Some(PathBuf::from(normalize_extended_prefix(display.as_ref())));
    }
}

#[cfg(windows)]
fn normalize_extended_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

#[cfg(windows)]
fn normalize_for_compare(path: &Path) -> String {
    let display = path.to_string_lossy();
    let text = normalize_extended_prefix(display.as_ref());
    text.replace('\\', "/").to_ascii_lowercase()
}

#[cfg(windows)]
struct ProcessAccumulator {
    exe_path: PathBuf,
    handles: BTreeSet<PathBuf>,
}

#[cfg(windows)]
struct WindowsProcessCollector {
    target_compare: String,
    processes: BTreeMap<u32, ProcessAccumulator>,
}

#[cfg(windows)]
impl WindowsProcessCollector {
    fn new(target: &Path) -> Self {
        Self {
            target_compare: normalize_for_compare(target),
            processes: BTreeMap::new(),
        }
    }

    fn add_handle(&mut self, pid: u32, exe_path: &Path, handle_path: PathBuf) {
        if !self.matches_target(&handle_path) {
            return;
        }

        let entry = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessAccumulator {
                exe_path: exe_path.to_path_buf(),
                handles: BTreeSet::new(),
            });

        if entry.exe_path.as_os_str().is_empty() {
            entry.exe_path = exe_path.to_path_buf();
        }

        entry.handles.insert(handle_path);
    }

    fn matches_target(&self, candidate: &Path) -> bool {
        normalize_for_compare(candidate).starts_with(&self.target_compare)
    }

    fn finish(self) -> Vec<ProcessInfo> {
        self.processes
            .into_iter()
            .map(|(pid, acc)| ProcessInfo {
                pid,
                exe_path: acc.exe_path,
                handles: acc.handles.into_iter().collect(),
            })
            .collect()
    }
}

/// Execute a command, replacing the current process on Unix
#[cfg(unix)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::os::unix::process::CommandExt;

    // exec() only returns on error
    Command::new(program).args(args).exec()
}

/// Execute a command on Windows (cannot replace process)
#[cfg(windows)]
pub fn exec_replace(program: &Path, args: Vec<OsString>) -> std::io::Error {
    use std::process::Stdio;

    match Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => {
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(e) => e,
    }
}

/// Launch a shell with environment variable set on Unix
#[cfg(unix)]
pub fn launch_shell_with_env(shell_path: &PathBuf, env_name: &str, env_value: &str) -> Result<()> {
    use std::os::unix::process::CommandExt;

    // Build command with environment variable
    // Parent process environment is inherited by default
    let err = Command::new(shell_path).env(env_name, env_value).exec();

    // exec only returns on error
    Err(KopiError::SystemError(format!(
        "Failed to execute shell: {err}"
    )))
}

/// Launch a shell with environment variable set on Windows
#[cfg(windows)]
pub fn launch_shell_with_env(shell_path: &PathBuf, env_name: &str, env_value: &str) -> Result<()> {
    use std::process::Stdio;

    // On Windows, we can't replace the process, so spawn and wait
    let status = Command::new(shell_path)
        .env(env_name, env_value)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| KopiError::SystemError(format!("Failed to spawn shell: {e}")))?;

    // Exit with the same code as the shell
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::KopiError;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn normalize_target_returns_canonical_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let nested = temp_dir.path().join("nested");
        fs::create_dir(&nested).expect("create nested dir");

        let normalized = normalize_target(&nested).expect("normalize succeeds");
        let expected = fs::canonicalize(&nested).expect("canonical path");

        assert_eq!(normalized, expected);
    }

    #[test]
    fn normalize_target_rejects_missing_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing = temp_dir.path().join("missing");

        let err = normalize_target(&missing).expect_err("expected error for missing path");
        match err {
            KopiError::DirectoryNotFound(message) => {
                assert!(message.contains("missing"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn normalize_target_rejects_file_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"data").expect("write test file");

        let err = normalize_target(&file_path).expect_err("expected validation error");
        match err {
            KopiError::ValidationError(message) => {
                assert!(message.contains("must be a directory"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    #[test]
    fn processes_using_path_returns_empty_vec_for_temp_dir() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let processes = processes_using_path(temp_dir.path()).expect("expected success");
        assert!(processes.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_fixture_enumeration_matches_snapshot() {
        use serde::Deserialize;
        use std::collections::BTreeMap;
        use std::os::unix::fs::symlink;

        #[derive(Deserialize)]
        struct FixturePath {
            path: String,
            #[serde(default = "FixturePath::default_scope")]
            scope: String,
            #[serde(default = "FixturePath::default_kind")]
            kind: String,
        }

        impl FixturePath {
            fn default_scope() -> String {
                "target".to_string()
            }

            fn default_kind() -> String {
                "file".to_string()
            }
        }

        #[derive(Deserialize)]
        struct FixtureProcess {
            pid: u32,
            exe: FixturePath,
            handles: Vec<FixturePath>,
        }

        #[derive(Deserialize)]
        struct Fixture {
            #[serde(rename = "note")]
            _note: String,
            target: String,
            processes: Vec<FixtureProcess>,
        }

        // Fixture derived from: ls -l /proc/*/fd | grep temurin-21.0.3 (captured 2025-10-20)
        let fixture_path = Path::new("tests/fixtures/linux_proc_fd_snapshot.json");
        let content = fs::read_to_string(fixture_path).expect("fixture readable");
        let fixture: Fixture = serde_json::from_str(&content).expect("fixture deserializable");

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let proc_root = temp_dir.path().join("proc");
        let target_root = temp_dir.path().join("targets");
        let external_root = temp_dir.path().join("external");
        fs::create_dir_all(&proc_root).expect("create proc root");
        fs::create_dir_all(&target_root).expect("create target root");
        fs::create_dir_all(&external_root).expect("create external root");

        let target_dir = target_root.join(&fixture.target);
        fs::create_dir_all(&target_dir).expect("create target path");
        let target_dir = fs::canonicalize(&target_dir).expect("canonical target");
        let canonical_target = target_dir.clone();

        let mut expected: BTreeMap<u32, (PathBuf, Vec<PathBuf>)> = BTreeMap::new();

        for process in &fixture.processes {
            let pid_dir = proc_root.join(process.pid.to_string());
            let fd_dir = pid_dir.join("fd");
            fs::create_dir_all(&fd_dir).expect("create fd dir");

            let exe_path = materialize_fixture_path(
                &process.exe,
                &target_dir,
                &external_root,
                process.pid,
                "exe",
            );
            let exe_link = pid_dir.join("exe");
            symlink(&exe_path, &exe_link).expect("create exe symlink");

            let mut recorded_handles: Vec<PathBuf> = Vec::new();
            for (handle_index, handle) in process.handles.iter().enumerate() {
                let resolved = materialize_fixture_path(
                    handle,
                    &target_dir,
                    &external_root,
                    process.pid,
                    "handle",
                );
                let fd_path = fd_dir.join(handle_index.to_string());
                symlink(&resolved, &fd_path).expect("create fd symlink");

                if resolved.starts_with(&canonical_target) {
                    recorded_handles.push(resolved);
                }
            }

            if !recorded_handles.is_empty() {
                expected.insert(process.pid, (exe_path.clone(), recorded_handles));
            }
        }

        let mut processes = linux_processes_using_path_with_root(&proc_root, &canonical_target)
            .expect("enumeration succeeds");
        processes.sort_by_key(|info| info.pid);

        assert_eq!(processes.len(), expected.len());

        for process in processes {
            let Some((exe, handles)) = expected.get(&process.pid) else {
                panic!("unexpected pid {} in results", process.pid);
            };

            assert_eq!(&process.exe_path, exe);
            assert_eq!(process.handles, *handles);
        }

        fn materialize_fixture_path(
            descriptor: &FixturePath,
            target_dir: &Path,
            external_root: &Path,
            pid: u32,
            kind: &str,
        ) -> PathBuf {
            let base = match descriptor.scope.as_str() {
                "target" => target_dir.join(&descriptor.path),
                _ => external_root
                    .join(format!("pid-{pid}"))
                    .join(&descriptor.path),
            };

            if let Some(parent) = base.parent() {
                fs::create_dir_all(parent).expect("create parent dirs");
            }

            match descriptor.kind.as_str() {
                "dir" => {
                    fs::create_dir_all(&base).expect("create dir fixture");
                }
                _ => {
                    if !base.exists() {
                        fs::write(&base, format!("{kind}-{pid}")).expect("write file fixture");
                    }
                }
            }

            fs::canonicalize(base).expect("canonical fixture path")
        }
    }

    #[cfg(all(
        not(target_os = "linux"),
        not(target_os = "macos"),
        not(target_os = "windows")
    ))]
    #[test]
    fn processes_using_path_returns_not_implemented_for_current_platform() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let err = processes_using_path(temp_dir.path()).expect_err("expected placeholder error");
        assert!(matches!(err, KopiError::NotImplemented(_)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_fixture_handles_are_scoped_to_target_directory() {
        use libc::c_char;
        use serde::Deserialize;
        use std::collections::{BTreeMap, BTreeSet};

        #[derive(Deserialize)]
        struct FixtureEntry {
            command: String,
            pid: u32,
            path: String,
        }

        #[derive(Deserialize)]
        struct Fixture {
            entries: Vec<FixtureEntry>,
        }

        fn copy_path_into(buffer: &mut [c_char; 1024], path: &str) {
            let bytes = path.as_bytes();
            for (idx, byte) in bytes.iter().enumerate() {
                buffer[idx] = *byte as c_char;
            }
            if bytes.len() < buffer.len() {
                buffer[bytes.len()] = 0;
            }
        }

        // Fixture derived from: lsof -Fpcfn -- /Users/example/.kopi/jdks/temurin-21.0.3 (captured 2025-10-12)
        let fixture_path = Path::new("tests/fixtures/macos_pidfdinfo_sample.json");
        let content = fs::read_to_string(fixture_path).expect("fixture readable");
        let fixture: Fixture = serde_json::from_str(&content).expect("fixture deserializable");

        let target = PathBuf::from("/Users/example/.kopi/jdks/temurin-21.0.3");

        let mut command_by_pid: BTreeMap<u32, String> = BTreeMap::new();
        let mut handles_by_pid: BTreeMap<u32, BTreeSet<PathBuf>> = BTreeMap::new();

        for entry in &fixture.entries {
            let mut vnode = RawVnodeFDInfoWithPath::default();
            copy_path_into(&mut vnode.pvip.vip_path, &entry.path);

            if let Some(handle_path) = extract_vnode_path(&vnode.pvip.vip_path) {
                command_by_pid
                    .entry(entry.pid)
                    .or_insert_with(|| entry.command.clone());
                let handles = handles_by_pid.entry(entry.pid).or_default();
                record_handle_if_within_target(&target, entry.pid, &handle_path, handles);
            }
        }

        let mut processes: Vec<ProcessInfo> = handles_by_pid
            .into_iter()
            .filter_map(|(pid, handles)| {
                if handles.is_empty() {
                    return None;
                }

                let command = command_by_pid
                    .remove(&pid)
                    .unwrap_or_else(|| "unknown".to_string());

                Some(ProcessInfo {
                    pid,
                    exe_path: PathBuf::from(format!("/mock/bin/{command}")),
                    handles: handles.into_iter().collect(),
                })
            })
            .collect();

        processes.sort_by_key(|info| info.pid);

        assert_eq!(processes.len(), 2);
        for process in &processes {
            assert!(
                process
                    .handles
                    .iter()
                    .all(|handle| handle.starts_with(&target)),
                "found handle outside target for pid {}",
                process.pid
            );
        }

        assert_eq!(
            processes[0].handles,
            vec![PathBuf::from(
                "/Users/example/.kopi/jdks/temurin-21.0.3/bin/java"
            )]
        );
        assert_eq!(
            processes[1].handles,
            vec![PathBuf::from(
                "/Users/example/.kopi/jdks/temurin-21.0.3/lib/tools.jar"
            )]
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_collector_filters_and_deduplicates_handles() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct FixtureProcess {
            pid: u32,
            exe: String,
            handles: Vec<String>,
        }

        #[derive(Deserialize)]
        struct Fixture {
            target: String,
            processes: Vec<FixtureProcess>,
        }

        // Fixture derived from: handle.exe -nobanner -a > capture.txt (recorded 2025-10-24)
        let fixture_path = Path::new("tests/fixtures/windows_handle_fixture.json");
        let content = fs::read_to_string(fixture_path).expect("fixture readable");
        let fixture: Fixture = serde_json::from_str(&content).expect("fixture deserializable");

        let target_path = PathBuf::from(&fixture.target);
        let mut collector = WindowsProcessCollector::new(&target_path);

        for process in &fixture.processes {
            let exe_path = PathBuf::from(&process.exe);
            for handle in &process.handles {
                collector.add_handle(process.pid, &exe_path, PathBuf::from(handle));
            }
        }

        let mut processes = collector.finish();
        processes.sort_by_key(|info| info.pid);

        assert_eq!(processes.len(), 2);

        let first = &processes[0];
        assert_eq!(first.pid, 4321);
        assert_eq!(
            first.exe_path,
            PathBuf::from("C:\\Program Files\\Java\\bin\\java.exe")
        );
        assert_eq!(
            first.handles,
            vec![
                PathBuf::from(
                    "C:\\Users\\example\\AppData\\Local\\Kopi\\jdks\\temurin-21.0.3\\bin\\java.exe"
                ),
                PathBuf::from(
                    "C:\\Users\\example\\AppData\\Local\\Kopi\\jdks\\temurin-21.0.3\\lib\\modules"
                )
            ]
        );

        let second = &processes[1];
        assert_eq!(second.pid, 5020);
        assert_eq!(
            second.exe_path,
            PathBuf::from("C:\\Tools\\Gradle\\bin\\gradle.exe")
        );
        assert_eq!(second.handles.len(), 2);
        for path in &second.handles {
            assert!(path.starts_with(&target_path));
        }
    }
}
