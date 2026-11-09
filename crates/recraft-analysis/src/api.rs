// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! API and filesystem-path classification used for malware-triage tags.
//!
//! Deliberately small and rule-based for M1: exact matches for short POSIX names,
//! substring matches for everything else, ordered from most specific to most generic.
//! This is a heuristic, not a precise taxonomy — the categories are meant to drive
//! filters in the UI/CLI, not to make claims about behaviour.

/// Short POSIX calls that are prefixes of Win32 APIs (`open` vs `OpenProcess`, `read`
/// vs `ReadFile`). Substring matching would misfile them, so they are matched against
/// the whole name first, ignoring an `@GLIBC_2.2.5`-style version suffix.
const POSIX_EXACT: &[(&str, &str)] = &[
    // File descriptors and stdio primitives (`fopen`/`fread`/... stay in the substring
    // table below).
    ("OPEN", "file"),
    ("OPEN64", "file"),
    ("OPENAT", "file"),
    ("OPENAT64", "file"),
    ("CREAT", "file"),
    ("CREAT64", "file"),
    ("READ", "file"),
    ("READ64", "file"),
    ("WRITE", "file"),
    ("WRITE64", "file"),
    ("PREAD", "file"),
    ("PREAD64", "file"),
    ("PWRITE", "file"),
    ("PWRITE64", "file"),
    ("READV", "file"),
    ("WRITEV", "file"),
    ("CLOSE", "file"),
    ("DUP", "file"),
    ("DUP2", "file"),
    ("FCNTL", "file"),
    ("STAT", "file"),
    ("FSTAT", "file"),
    ("LSTAT", "file"),
    ("FSTATAT", "file"),
    ("ACCESS", "file"),
    ("FSYNC", "file"),
    ("FDATASYNC", "file"),
    ("FTRUNCATE", "file"),
    ("RMDIR", "file"),
    ("CHDIR", "file"),
    ("GETCWD", "file"),
    ("READLINK", "file"),
    ("SYMLINK", "file"),
    ("MKDIRAT", "file"),
    ("UNLINKAT", "file"),
    ("RENAMEAT", "file"),
    // Dynamic loading and process control beyond the substring table.
    ("DLSYM", "process"),
    ("DLCLOSE", "process"),
    ("CLONE", "process"),
    ("MPROTECT", "memory"),
    ("BRK", "memory"),
    ("SBRK", "memory"),
];

/// Categorize an imported API name. Returns `"other"` when no rule matches.
pub fn categorize(api: &str) -> &'static str {
    let name = api.trim().to_ascii_uppercase();
    // Versioned ELF symbols read `open@GLIBC_2.2.5`; the version is not behaviour.
    let base = name.split('@').next().unwrap_or(&name);
    if let Some((_, category)) = POSIX_EXACT.iter().find(|(api, _)| *api == base) {
        return category;
    }
    const RULES: &[(&str, &[&str])] = &[
        (
            "anti-analysis",
            &[
                "ISDEBUGGERPRESENT",
                "CHECKREMOTEDEBUGGERPRESENT",
                "NTQUERYINFORMATIONPROCESS",
                "OUTPUTDEBUGSTRING",
                "GETTICKCOUNT",
                "QUERYPERFORMANCECOUNTER",
                "GETSYSTEMTIMEASFILETIME",
                "NTSETINFORMATIONTHREAD",
                // POSIX: a tracer attached to this process is being detected/evaded.
                "PTRACE",
                "PRCTL",
            ],
        ),
        (
            "network",
            &[
                "WSA",
                "SOCKET",
                "CONNECT",
                "SEND",
                "RECV",
                "BIND",
                "LISTEN",
                "ACCEPT",
                "INTERNET",
                "HTTP",
                "WINHTTP",
                "WININET",
                "DNS",
                "URLDOWNLOAD",
                "GETADDRINFO",
                "GETHOSTBYNAME",
            ],
        ),
        // Registry APIs all start with a `Reg` verb, but the 3-letter prefix alone also
        // fires on unrelated ELF symbols (`_ITM_deregisterTMCloneTable`) and on
        // `RegisterClass`, so match the verb instead of the prefix.
        (
            "registry",
            &[
                "REGOPEN",
                "REGCREATE",
                "REGSET",
                "REGGET",
                "REGDELETE",
                "REGQUERY",
                "REGENUM",
                "REGCLOSE",
                "REGLOAD",
                "REGUNLOAD",
                "REGFREE",
                "REGFLUSH",
                "REGNOTIFY",
                "REGCONNECT",
                "REGSAVE",
                "REGRESTORE",
            ],
        ),
        (
            "crypto",
            &["CRYPT", "BCRYPT", "NCRYPT", "CERT", "OPENSSL", "EVP_"],
        ),
        (
            "service",
            &[
                "OPENSCMANAGER",
                "CREATESERVICE",
                "OPENSERVICE",
                "STARTSERVICE",
                "CONTROLSERVICE",
            ],
        ),
        (
            "thread",
            &[
                "CREATETHREAD",
                "CREATEREMOTETHREAD",
                "WAITFORSINGLEOBJECT",
                "SLEEP",
                "CREATEMUTEX",
                "CREATEEVENT",
                "PTHREAD",
            ],
        ),
        (
            "process",
            &[
                "CREATEPROCESS",
                "OPENPROCESS",
                "TERMINATEPROCESS",
                "OPENPROCESS",
                "WRITEPROCESSMEMORY",
                "READPROCESSMEMORY",
                "VIRTUALALLOC",
                "VIRTUALPROTECT",
                "VIRTUALFREE",
                "LOADLIBRARY",
                "GETPROCADDRESS",
                "FREELIBRARY",
                "WINEXEC",
                "SHELLEXECUTE",
                // POSIX: fork/exec, signals and dynamic loading.
                "FORK",
                "EXEC",
                "SYSTEM",
                "WAITPID",
                "KILL",
                "DLOPEN",
            ],
        ),
        (
            "file",
            &[
                "CREATEFILE",
                "READFILE",
                "WRITEFILE",
                "DELETEFILE",
                "MOVEFILE",
                "COPYFILE",
                "FINDFIRSTFILE",
                "FINDNEXTFILE",
                "GETTEMPPATH",
                "GETTEMPFILENAME",
                "SETFILEATTRIBUTES",
                "SETFILEPOINTER",
                "CREATEDIRECTORY",
                // POSIX stdio and directory calls.
                "FOPEN",
                "FREAD",
                "FWRITE",
                "FCLOSE",
                "UNLINK",
                "RENAME",
                "MKDIR",
                "OPENDIR",
                "READDIR",
                "CHMOD",
                "TRUNCATE",
                "LSEEK",
            ],
        ),
        (
            "ui",
            &[
                "MESSAGEBOX",
                "CREATEWINDOW",
                "SHOWWINDOW",
                "DIALOGBOX",
                "SETWINDOWTEXT",
                "REGISTERCLASS",
                "GETMESSAGE",
                "TRANSLATEMESSAGE",
                "DISPATCHMESSAGE",
            ],
        ),
        (
            "memory",
            &[
                "HEAPALLOC",
                "HEAPFREE",
                "LOCALALLOC",
                "GLOBALALLOC",
                "MALLOC",
                "FREE",
                "CALLOC",
                "REALLOC",
                // POSIX memory mapping.
                "MMAP",
                "MUNMAP",
            ],
        ),
    ];

    for (category, needles) in RULES {
        if needles.iter().any(|needle| name.contains(needle)) {
            return category;
        }
    }
    "other"
}

/// Categorize a string literal that names a `/proc` path.
///
/// `/proc` access is behaviour rather than an import: Linux malware reads
/// `/proc/self/status` or `/proc/self/maps` to notice a debugger/sandbox, and
/// `/proc/<pid>/mem` to reach into another process. Reading the literals is the only
/// way to see that, so triage classifies them separately from API tags.
///
/// Returns `None` for anything that is not a `/proc` path.
pub fn categorize_path(text: &str) -> Option<&'static str> {
    let lower = text.trim().to_ascii_lowercase();
    let rest = lower
        .strip_prefix("/proc")
        .and_then(|rest| rest.strip_prefix('/'))?;
    if rest.is_empty() {
        return Some("file");
    }
    // Self-inspection: the classic anti-debug / sandbox probe.
    if rest == "self"
        || rest == "thread-self"
        || rest.starts_with("self/")
        || rest.starts_with("thread-self/")
    {
        return Some("anti-analysis");
    }
    // `/proc/net/*` is the routing/socket table, not a file.
    if rest == "net" || rest.starts_with("net/") {
        return Some("network");
    }
    // `/proc/<pid>/{maps,mem,status,fd,...}` reaches into another process.
    if let Some((pid, tail)) = rest.split_once('/')
        && !pid.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
    {
        let head = tail.split('/').next().unwrap_or(tail);
        if matches!(
            head,
            "maps" | "mem" | "status" | "stat" | "fd" | "task" | "exe" | "cmdline" | "environ"
        ) {
            return Some("process");
        }
    }
    Some("file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_common_apis() {
        assert_eq!(categorize("CreateFileW"), "file");
        assert_eq!(categorize("WSAStartup"), "network");
        assert_eq!(categorize("InternetOpenUrlW"), "network");
        assert_eq!(categorize("RegOpenKeyExW"), "registry");
        assert_eq!(categorize("CreateRemoteThread"), "thread");
        assert_eq!(categorize("IsDebuggerPresent"), "anti-analysis");
        assert_eq!(categorize("MessageBoxW"), "ui");
        assert_eq!(categorize("VirtualAlloc"), "process");
        assert_eq!(categorize("CryptAcquireContextW"), "crypto");
        assert_eq!(categorize("SomeUnknownApi"), "other");
    }

    #[test]
    fn categorizes_common_posix_apis() {
        assert_eq!(categorize("ptrace"), "anti-analysis");
        assert_eq!(categorize("socket"), "network");
        assert_eq!(categorize("connect"), "network");
        assert_eq!(categorize("fork"), "process");
        assert_eq!(categorize("execve"), "process");
        assert_eq!(categorize("dlopen"), "process");
        assert_eq!(categorize("pthread_create"), "thread");
        assert_eq!(categorize("fopen"), "file");
        assert_eq!(categorize("opendir"), "file");
        assert_eq!(categorize("mmap"), "memory");
        assert_eq!(categorize("malloc"), "memory");
        assert_eq!(categorize("EVP_EncryptInit"), "crypto");
    }

    /// The bare descriptor calls the Linux table was missing: `open`/`write`/`read`
    /// only matched the stdio `fopen`-style names before, and every one of them fell
    /// through to `other`.
    #[test]
    fn categorizes_bare_posix_descriptor_calls() {
        assert_eq!(categorize("open"), "file");
        assert_eq!(categorize("open64"), "file");
        assert_eq!(categorize("openat"), "file");
        assert_eq!(categorize("write"), "file");
        assert_eq!(categorize("read"), "file");
        assert_eq!(categorize("pread"), "file");
        assert_eq!(categorize("close"), "file");
        assert_eq!(categorize("stat"), "file");
        assert_eq!(categorize("ftruncate"), "file");
        assert_eq!(categorize("dlsym"), "process");
        assert_eq!(categorize("mprotect"), "memory");
        // Versioned ELF symbol names lose their suffix.
        assert_eq!(categorize("read@GLIBC_2.2.5"), "file");
    }

    /// Exact matching must not misfile the Win32 APIs whose names merely start with a
    /// POSIX verb.
    #[test]
    fn exact_rules_do_not_shadow_win32_names() {
        assert_eq!(categorize("OpenProcess"), "process");
        assert_eq!(categorize("OpenServiceW"), "service");
        assert_eq!(categorize("ReadFile"), "file");
        assert_eq!(categorize("CloseHandle"), "other");
    }

    /// The 3-letter `Reg` prefix used to tag any symbol that merely contains `reg`:
    /// ELF's `_ITM_deregisterTMCloneTable` was reported as `registry`, and
    /// `RegisterClassW` could never reach the `ui` rule behind it.
    #[test]
    fn registry_rules_need_a_registry_verb() {
        assert_eq!(categorize("RegCloseKey"), "registry");
        assert_eq!(categorize("RegSetValueExW"), "registry");
        assert_eq!(categorize("_ITM_deregisterTMCloneTable"), "other");
        assert_eq!(categorize("__register_frame_info"), "other");
        assert_eq!(categorize("RegisterClassW"), "ui");
    }

    #[test]
    fn categorizes_proc_paths() {
        assert_eq!(categorize_path("/proc/self/status"), Some("anti-analysis"));
        assert_eq!(categorize_path("/proc/self/maps"), Some("anti-analysis"));
        assert_eq!(
            categorize_path("/proc/thread-self/stat"),
            Some("anti-analysis")
        );
        assert_eq!(categorize_path("/proc/1234/mem"), Some("process"));
        assert_eq!(categorize_path("/proc/1/maps"), Some("process"));
        assert_eq!(categorize_path("/proc/net/tcp"), Some("network"));
        assert_eq!(categorize_path("/proc/cpuinfo"), Some("file"));
        assert_eq!(categorize_path("/etc/passwd"), None);
        assert_eq!(categorize_path("https://example.invalid/proc/self"), None);
        assert_eq!(categorize_path(""), None);
    }
}
