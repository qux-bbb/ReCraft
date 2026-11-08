// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! API categorization rules used for malware-triage tags.
//!
//! Deliberately small and rule-based for M1: substring matches on the upper-cased
//! import name. Rules are ordered from most specific to most generic. This is a
//! heuristic, not a precise taxonomy — the categories are meant to drive filters in
//! the UI/CLI, not to make claims about behaviour.

/// Categorize an imported API name. Returns `"other"` when no rule matches.
pub fn categorize(api: &str) -> &'static str {
    let name = api.to_ascii_uppercase();
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
        ("registry", &["REG"]),
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
}
