// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

// M1 corpus: C++ exports with MSVC-decorated names, used to validate demangling.

typedef unsigned int u32;

// Keep floating point from pulling in the CRT (the double overload uses SSE).
int _fltused = 1;

namespace recraft {

class __declspec(dllexport) DemoTarget {
public:
    DemoTarget();
    int get() const;
    void set(int value);
    static int counter();

private:
    int field_;
    static int instances_;
};

DemoTarget::DemoTarget() : field_(0) {
    instances_++;
}

int DemoTarget::get() const {
    return field_;
}

void DemoTarget::set(int value) {
    field_ = value;
}

int DemoTarget::instances_ = 0;

int DemoTarget::counter() {
    return instances_;
}

__declspec(dllexport) int plain(int value) {
    return value + 1;
}

__declspec(dllexport) int overloaded(int value) {
    return value * 2;
}

__declspec(dllexport) int overloaded(double value) {
    return static_cast<int>(value);
}

template <typename T>
static T twice(T value) {
    return value + value;
}

__declspec(dllexport) int twice_int(int value) {
    return twice(value);
}

} // namespace recraft

extern "C" __declspec(dllexport) int plain_c(int left, int right) {
    return left + right;
}
