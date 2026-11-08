/*
 * SPDX-License-Identifier: AGPL-3.0-only
 * Copyright (c) 2026 ReCraft contributors
 *
 * Linux ELF corpus: the same shapes as `corpus/c/functions.c` (leaf arithmetic,
 * branches, loops, switches, recursion, indirect calls, struct/array probes,
 * bit operations, strings) plus libc calls, so the ELF loader sees real PLT/GOT
 * relocations and the decompiler sees System V calling conventions.
 *
 * Built by `corpus/build-linux.sh` as a shared object:
 *   gcc -O2 -fPIC -shared -o corpus/build/linux/libcorpus.so functions.c
 *
 * The shared object is a build product and is never committed.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned int u32;
typedef unsigned char u8;

int add_leaf(int a, int b) {
    return a + b;
}

int branchy(int x) {
    if (x > 0) {
        if (x > 100) {
            return 3;
        }
        return 1;
    }
    return x == 0 ? 0 : 2;
}

int loopy(int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        total += i * 3;
    }
    return total;
}

int switcher(int x) {
    switch (x) {
    case 1:
        return 10;
    case 2:
        return 20;
    case 3:
        return 30;
    default:
        return -1;
    }
}

int switcher_big(int x) {
    switch (x) {
    case 0:
        return 100;
    case 1:
        return 101;
    case 2:
        return 102;
    case 3:
        return 103;
    case 4:
        return 104;
    case 5:
        return 105;
    case 6:
        return 106;
    case 7:
        return 107;
    default:
        return 0;
    }
}

int caller(int x) {
    return add_leaf(x, 3);
}

int recursive(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * recursive(n - 1);
}

int indirect(int (*fn)(int, int), int a, int b) {
    return fn(a, b);
}

int nested_loops(int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            if ((i ^ j) & 1) {
                total += i + j;
            }
        }
    }
    return total;
}

int tailish(int n) {
    while (n > 0) {
        n -= 2;
    }
    return n;
}

int float_math(float value) {
    return (int)(value * 2.0f) + 1;
}

int bitops(u32 x) {
    u32 y = x | 0x10u;
    y &= 0xffu;
    y ^= 3u;
    return (int)(y << 2);
}

int bytes_op(u8 *data, int len) {
    int sum = 0;
    for (int i = 0; i < len; i++) {
        sum += data[i];
    }
    return sum;
}

__attribute__((noinline)) int noinline_helper(int a, int b) {
    return a * 3 + b;
}

int uses_helper(int a) {
    return noinline_helper(a, a + 1);
}

const char *corpus_banner(void) {
    return "ReCraft ELF corpus";
}

static int counter = 40;

int corpus_counter_bump(void) {
    return ++counter;
}

__attribute__((noinline)) static int handler_a(int value) {
    return value + 1;
}

__attribute__((noinline)) static int handler_b(int value) {
    return value * 2;
}

__attribute__((noinline)) static int handler_c(int value) {
    return value - 3;
}

__attribute__((noinline)) static int handler_d(int value) {
    return value ^ 0x55;
}

int dispatch(int index) {
    static int (*handlers[])(int) = {handler_a, handler_b, handler_c, handler_d};
    if (index < 0 || index >= 4) {
        return -1;
    }
    return handlers[index](index);
}

int str_probe(const char *text) {
    return (int)strlen(text);
}

int call_str(const char *text) {
    return str_probe(text) + 1;
}

int field_probe(const int *p) {
    return p[0] + p[1] + p[2];
}

int array_probe(const int *p, int i) {
    return p[i] + i;
}

int early_chain(int a, int b) {
    if (a > b) {
        return a - b;
    }
    return b - a;
}

int count_bits(u32 value) {
    int count = 0;
    while (value != 0) {
        value &= value - 1;
        count++;
    }
    return count;
}

/* Real libc imports: printf/malloc/free/memcpy come through the PLT and GOT. */
int libc_probe(void) {
    char *buffer = malloc(16);
    if (buffer == NULL) {
        return -1;
    }
    memcpy(buffer, "elf", 4);
    int length = (int)strlen(buffer);
    printf("%s\n", buffer);
    free(buffer);
    return length;
}

int getenv_probe(void) {
    const char *value = getenv("RECRAFT_CORPUS");
    return value == NULL ? 0 : (int)strlen(value);
}
