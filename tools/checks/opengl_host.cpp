// SPDX-License-Identifier: BSD-3-Clause
// Adapted from SM2-Emu Libretro opengl_host.cpp; same frontend lifecycle gates.
// Minimal surfaceless OpenGL/OpenGL ES frontend for Libretro lifecycle/image checks.
#include "libretro_hw.h"

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl31.h>

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {
retro_hw_render_callback* callback = nullptr;
EGLDisplay display = EGL_NO_DISPLAY;
EGLSurface surface = EGL_NO_SURFACE;
EGLContext context = EGL_NO_CONTEXT;
std::vector<unsigned char> pixels;
std::string error;
bool use_es = true;

uintptr_t current_framebuffer() { return 0; }
retro_proc_address_t get_proc(const char* name)
{
    return reinterpret_cast<retro_proc_address_t>(eglGetProcAddress(name));
}
void set_error(const char* operation)
{
    error = std::string(operation) + " (EGL 0x";
    char code[16]{};
    std::snprintf(code, sizeof(code), "%04x", eglGetError());
    error += code;
    error += ")";
}
void destroy_egl(bool notify)
{
    if (display != EGL_NO_DISPLAY) {
        if (notify && callback && callback->context_destroy) callback->context_destroy();
        eglMakeCurrent(display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        if (context != EGL_NO_CONTEXT) eglDestroyContext(display, context);
        if (surface != EGL_NO_SURFACE) eglDestroySurface(display, surface);
        eglTerminate(display);
    }
    display = EGL_NO_DISPLAY;
    surface = EGL_NO_SURFACE;
    context = EGL_NO_CONTEXT;
    pixels.clear();
}
}

extern "C" {
void tgpulse_gl_set_es(bool es) { use_es = es; }

bool tgpulse_gl_environment(unsigned command, void* data)
{
    if (command == RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER) {
        *static_cast<unsigned*>(data) = use_es ? RETRO_HW_CONTEXT_OPENGLES_VERSION
                                              : RETRO_HW_CONTEXT_OPENGL_CORE;
        return true;
    }
    if (command != RETRO_ENVIRONMENT_SET_HW_RENDER) return false;
    callback = static_cast<retro_hw_render_callback*>(data);
    const bool valid_es = use_es && callback->context_type == RETRO_HW_CONTEXT_OPENGLES_VERSION &&
                          callback->version_major == 3 && callback->version_minor >= 1;
    const bool valid_desktop = !use_es && callback->context_type == RETRO_HW_CONTEXT_OPENGL_CORE &&
                               callback->version_major == 4 && callback->version_minor >= 3;
    if (!valid_es && !valid_desktop)
        return false;
    callback->get_current_framebuffer = current_framebuffer;
    callback->get_proc_address = get_proc;
    return true;
}

const char* tgpulse_gl_error() { return error.c_str(); }

bool tgpulse_gl_start()
{
    error.clear();
    display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if (display == EGL_NO_DISPLAY) { set_error("eglGetDisplay"); return false; }
    EGLint major = 0, minor = 0;
    if (!eglInitialize(display, &major, &minor)) { set_error("eglInitialize"); return false; }
    if (!eglBindAPI(use_es ? EGL_OPENGL_ES_API : EGL_OPENGL_API)) {
        set_error("eglBindAPI"); destroy_egl(false); return false;
    }
    const EGLint config_attributes[] = {
        EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
        EGL_RENDERABLE_TYPE, use_es ? EGL_OPENGL_ES3_BIT : EGL_OPENGL_BIT,
        EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8,
        EGL_NONE};
    EGLConfig config = nullptr;
    EGLint count = 0;
    if (!eglChooseConfig(display, config_attributes, &config, 1, &count) || count != 1) {
        set_error("eglChooseConfig"); destroy_egl(false); return false;
    }
    const EGLint surface_attributes[] = {EGL_WIDTH, 1984, EGL_HEIGHT, 1536, EGL_NONE};
    surface = eglCreatePbufferSurface(display, config, surface_attributes);
    if (surface == EGL_NO_SURFACE) { set_error("eglCreatePbufferSurface"); destroy_egl(false); return false; }
    const EGLint es_context_attributes[] = {
        EGL_CONTEXT_MAJOR_VERSION_KHR, 3, EGL_CONTEXT_MINOR_VERSION_KHR, 1, EGL_NONE};
    const EGLint desktop_context_attributes[] = {
        EGL_CONTEXT_MAJOR_VERSION_KHR, 4,
        EGL_CONTEXT_MINOR_VERSION_KHR, 3,
        EGL_CONTEXT_OPENGL_PROFILE_MASK_KHR, EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT_KHR,
        EGL_NONE};
    const EGLint* context_attributes = use_es ? es_context_attributes
                                              : desktop_context_attributes;
    context = eglCreateContext(display, config, EGL_NO_CONTEXT, context_attributes);
    if (context == EGL_NO_CONTEXT) {
        set_error(use_es ? "eglCreateContext ES 3.1" : "eglCreateContext OpenGL 4.3 core");
        destroy_egl(false);
        return false;
    }
    if (!eglMakeCurrent(display, surface, surface, context)) {
        set_error("eglMakeCurrent"); destroy_egl(false); return false;
    }
    if (!callback || !callback->context_reset) {
        error = "Core did not register OpenGL context callbacks";
        destroy_egl(false);
        return false;
    }
    callback->context_reset();
    return true;
}

void tgpulse_gl_stop() { destroy_egl(true); }
void tgpulse_gl_lose_context() { destroy_egl(false); }

bool tgpulse_gl_frame(unsigned width, unsigned height, bool capture)
{
    glFinish();
    if (glGetError() != GL_NO_ERROR) { error = "OpenGL error after core frame"; return false; }
    if (capture) {
        pixels.resize(static_cast<size_t>(width) * height * 4);
        glPixelStorei(GL_PACK_ALIGNMENT, 1);
        glReadPixels(0, 0, static_cast<GLsizei>(width), static_cast<GLsizei>(height),
                     GL_RGBA, GL_UNSIGNED_BYTE, pixels.data());
        if (glGetError() != GL_NO_ERROR) { error = "OpenGL readback failed"; return false; }
    }
    return true;
}

const void* tgpulse_gl_pixels() { return pixels.data(); }
}
