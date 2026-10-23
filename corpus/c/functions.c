/*
 * SPDX-License-Identifier: AGPL-3.0-only
 * Copyright (c) 2026 ReCraft contributors
 *
 * M1 analysis corpus: a CRT-free PE where every function is exported, so the
 * export table + .pdata are an unbiased ground truth for function discovery.
 *
 * Built by corpus/build.ps1 with MSVC:
 *   cl /O2 /GS- /c functions.c
 *   link /NODEFAULTLIB /ENTRY:corpus_main /EXPORT:... /SUBSYSTEM:CONSOLE
 */

typedef unsigned int u32;
typedef unsigned char u8;
typedef int (*binary_fn)(int, int);
typedef int (*unary_fn)(int);

#include <stddef.h> /* wchar_t, headers only - no CRT is linked */

/* Keep floating point from pulling in the CRT. */
int _fltused = 1;

__declspec(dllexport) int add_leaf(int a, int b) {
    return a + b;
}

__declspec(dllexport) int branchy(int x) {
    if (x > 10) {
        return x * 2;
    } else if (x < -5) {
        return -x;
    }
    return 0;
}

__declspec(dllexport) int loopy(int n) {
    int sum = 0;
    for (int i = 0; i < n; i++) {
        sum += i;
    }
    return sum;
}

__declspec(dllexport) int switcher(int x) {
    switch (x) {
        case 0: return 10;
        case 1: return 20;
        case 2: return 30;
        case 3: return 40;
        case 4: return 50;
        default: return -1;
    }
}

/* Large enough that MSVC emits a jump table instead of a comparison chain. */
__declspec(dllexport) int switcher_big(int x) {
    switch (x) {
        case 0: return 100;
        case 1: return 101;
        case 2: return 102;
        case 3: return 103;
        case 4: return 104;
        case 5: return 105;
        case 6: return 106;
        case 7: return 107;
        case 8: return 108;
        case 9: return 109;
        case 10: return 110;
        case 11: return 111;
        case 12: return 112;
        case 13: return 113;
        case 14: return 114;
        case 15: return 115;
        case 16: return 116;
        case 17: return 117;
        case 18: return 118;
        case 19: return 119;
        default: return -1;
    }
}

__declspec(dllexport) int caller(int x) {
    return add_leaf(x, branchy(x));
}

__declspec(dllexport) int recursive(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * recursive(n - 1);
}

__declspec(dllexport) int indirect(binary_fn fn, int a, int b) {
    return fn(a, b);
}

__declspec(dllexport) int nested_loops(int n) {
    int sum = 0;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            sum += i * j;
        }
    }
    return sum;
}

__declspec(dllexport) int tailish(int n) {
    if (n == 0) {
        return 0;
    }
    return tailish(n - 1);
}

__declspec(dllexport) int float_math(float value) {
    return (int)(value * 2.5f + 1.0f);
}

__declspec(dllexport) int bitops(u32 x) {
    return (int)(((x << 3) ^ (x >> 2)) | 0xF0u);
}

__declspec(dllexport) int bytes_op(u8 *data, int len) {
    int sum = 0;
    for (int i = 0; i < len; i++) {
        sum += data[i];
    }
    return sum;
}

__declspec(dllexport) int noinline_helper(int a, int b) {
    /* Prevent inlining so it stays a separate, callable function. */
    volatile int x = a;
    volatile int y = b;
    return x * y + 7;
}

__declspec(dllexport) int uses_helper(int a) {
    return noinline_helper(a, a + 1);
}

/* Entry point: exported functions are retained by the linker via /EXPORT. */
void corpus_main(void) {
    volatile int x = 1;
    while (x) {
        x = 0;
    }
}

/* --- strings / data references (M1 string & xref evaluation) ---------------- */

__declspec(dllexport) const char *corpus_banner(void) {
    return "ReCraft analysis corpus";
}

__declspec(dllexport) const wchar_t *corpus_wide_banner(void) {
    return L"ReCraft wide corpus";
}

__declspec(dllexport) int corpus_counter_bump(void) {
    static int counter;
    counter++;
    return counter;
}
