/*
 * SPDX-License-Identifier: AGPL-3.0-only
 * Copyright (c) 2026 ReCraft contributors
 *
 * Linux behaviour sample: the counterpart of the API classification table.
 *
 * Each function calls one family of libc/syscall APIs that triage must recognise
 * (open/read/write, socket/connect, ptrace, dlopen) and references `/proc` paths that
 * the path classifier turns into behaviour tags. It is kept separate from
 * `functions.c` on purpose: that file backs the committed ELF golden snapshots, and
 * adding functions there would invalidate every one of them.
 *
 * Built by `corpus/build-linux.sh` as `corpus/build/linux/libcorpus-behaviors.so`
 * (a build product, never committed); the ELF triage test in `recraft-ai` skips when
 * the file is absent, so a clean Windows checkout stays green.
 */

#include <dlfcn.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stddef.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

/* open + read + close on a literal path: file APIs, `/proc/self` anti-analysis. */
int read_self_status(void) {
    char buffer[256];
    int fd = open("/proc/self/status", O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    ssize_t got = read(fd, buffer, sizeof buffer - 1);
    close(fd);
    return (int)got;
}

/* A second self-inspection path: maps is the classic memory-layout probe. */
int read_self_maps(void) {
    char buffer[512];
    int fd = open("/proc/self/maps", O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    ssize_t got = read(fd, buffer, sizeof buffer - 1);
    close(fd);
    return (int)got;
}

/* Another process: `/proc/<pid>/mem` is process access, not self-inspection. */
int read_other_process(void) {
    char buffer[64];
    int fd = open("/proc/1/mem", O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    ssize_t got = read(fd, buffer, sizeof buffer - 1);
    close(fd);
    return (int)got;
}

/* The write side of the file table: open(O_WRONLY) + write + close. */
int write_marker(void) {
    int fd = open("/tmp/recraft-marker", O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        return -1;
    }
    ssize_t wrote = write(fd, "recraft\n", 8);
    close(fd);
    return (int)wrote;
}

/* socket + connect: network classification. */
int connect_probe(void) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return -1;
    }
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof addr);
    addr.sin_family = AF_INET;
    addr.sin_port = 0x5000;            /* port 80, network byte order */
    addr.sin_addr.s_addr = 0x0100007f; /* 127.0.0.1 */
    int rc = connect(fd, (const struct sockaddr *)&addr, sizeof addr);
    close(fd);
    return rc;
}

/* `/proc/net/*` is the socket/routing table: a network path, not a file. */
int read_proc_net(void) {
    char buffer[128];
    int fd = open("/proc/net/tcp", O_RDONLY);
    if (fd < 0) {
        return -1;
    }
    ssize_t got = read(fd, buffer, sizeof buffer - 1);
    close(fd);
    return (int)got;
}

/* ptrace: anti-analysis, the POSIX counterpart of IsDebuggerPresent. */
int tracer_probe(void) {
    return ptrace(PTRACE_TRACEME, 0, NULL, NULL) == 0 ? 1 : 0;
}

/* dlopen + dlsym + dlclose: dynamic loading, classified as process behaviour. */
int load_libc(void) {
    void *handle = dlopen("libc.so.6", RTLD_LAZY);
    if (handle == NULL) {
        return 0;
    }
    void *symbol = dlsym(handle, "getpid");
    dlclose(handle);
    return symbol == NULL ? 0 : 1;
}
