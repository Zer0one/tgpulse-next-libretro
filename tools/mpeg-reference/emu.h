// Minimal compile-only MAME shim for the standalone MPEG oracle.
// No emulation framework or state-saving behavior is supplied by this stub.
#pragma once
#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
class device_t {
public:
    template <typename T> void save_item(T &, const char *, int) {}
};
