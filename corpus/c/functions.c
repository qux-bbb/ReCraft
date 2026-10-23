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
