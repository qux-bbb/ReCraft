/*
 * SPDX-License-Identifier: AGPL-3.0-only
 * Copyright (c) 2026 ReCraft contributors
 *
 * C++ ELF corpus: namespaced functions, a class with methods, overloads and a
 * template, so the loader sees Itanium-mangled `_Z...` names that must be
 * demangled before they reach the name table.
 *
 * Built by `corpus/build-linux.sh` (g++ -O2 -fPIC -shared).
 */

#include <cstdint>
#include <cstdio>

namespace recraft {

namespace detail {

int triple(int value) {
    return value * 3;
}

template <typename T>
T clamp_low(T value, T low) {
    return value < low ? low : value;
}

} // namespace detail

class DemoTarget {
public:
    explicit DemoTarget(int seed) : seed_(seed) {}

    int add(int value) const {
        return seed_ + value;
    }

    int get() const {
        return seed_;
    }

    void bump(int by) {
        seed_ += by;
    }

private:
    int seed_;
};

int square(int value) {
    return value * value;
}

double square(double value) {
    return value * value;
}

int combine(int a, int b) {
    return detail::triple(a) + b;
}

int clamped(int value) {
    return detail::clamp_low<int>(value, 0);
}

const char *tag() {
    return "recraft cpp corpus";
}

} // namespace recraft

extern "C" int cpp_entry(int value) {
    recraft::DemoTarget target(value);
    target.bump(1);
    std::printf("%s\n", recraft::tag());
    return target.add(2) + recraft::square(value) + recraft::clamped(value);
}
