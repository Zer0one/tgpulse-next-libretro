// Minimal public Libretro hardware callback ABI used only by the EGL check host.
#pragma once
#include <cstddef>
#include <cstdint>
using retro_proc_address_t = void (*)();
constexpr unsigned RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER = 56;
constexpr unsigned RETRO_ENVIRONMENT_SET_HW_RENDER = 14;
constexpr unsigned RETRO_HW_CONTEXT_OPENGL_CORE = 3;
constexpr unsigned RETRO_HW_CONTEXT_OPENGLES_VERSION = 5;
struct retro_hw_render_callback {
    unsigned context_type;
    void (*context_reset)();
    uintptr_t (*get_current_framebuffer)();
    retro_proc_address_t (*get_proc_address)(const char*);
    bool depth, stencil, bottom_left_origin;
    unsigned version_major, version_minor;
    bool cache_context;
    void (*context_destroy)();
    bool debug_context;
};
static_assert(sizeof(retro_hw_render_callback) == 64, "64-bit callback layout");
