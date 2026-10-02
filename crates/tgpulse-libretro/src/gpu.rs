//! Frontend GPU resources stay outside the emulated machine.
pub mod opengl;
pub mod vulkan;

use std::ffi::{c_char, c_void};
use tgpulse_core::model1_video::{gpu_quad_bins, GpuQuad};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HwCallback {
    pub context_type: u32,
    pub context_reset: Option<unsafe extern "C" fn()>,
    pub get_current_framebuffer: Option<unsafe extern "C" fn() -> usize>,
    pub get_proc_address: Option<unsafe extern "C" fn(*const c_char) -> *const c_void>,
    pub depth: bool,
    pub stencil: bool,
    pub bottom_left_origin: bool,
    pub version_major: u32,
    pub version_minor: u32,
    pub cache_context: bool,
    pub context_destroy: Option<unsafe extern "C" fn()>,
    pub debug_context: bool,
}
unsafe impl Send for HwCallback {}
impl HwCallback {
    pub fn vulkan(reset: unsafe extern "C" fn(), destroy: unsafe extern "C" fn()) -> Self {
        Self {
            context_type: 6,
            context_reset: Some(reset),
            get_current_framebuffer: None,
            get_proc_address: None,
            depth: false,
            stencil: false,
            bottom_left_origin: false,
            version_major: 0,
            version_minor: 0,
            cache_context: false,
            context_destroy: Some(destroy),
            debug_context: false,
        }
    }
}
pub const SET_HW_RENDER: u32 = 14;
pub const GET_HW_RENDER_INTERFACE: u32 = 41 | 0x10000;
pub const HW_FRAME: *const c_void = usize::MAX as *const c_void;

pub const RASTER: &str = include_str!("../../tgpulse/src/platform/gpu_model1.wgsl");
pub const RESOLVE: &str = include_str!("../../tgpulse/src/platform/gpu_resolve.wgsl");
pub fn shader(source: &str) -> Result<(naga::Module, naga::valid::ModuleInfo), String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| e.to_string())?;
    Ok((module, info))
}
pub fn spirv(source: &str, entry: &str) -> Result<Vec<u32>, String> {
    let (module, info) = shader(source)?;
    naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options::default(),
        Some(&naga::back::spv::PipelineOptions {
            shader_stage: naga::ShaderStage::Compute,
            entry_point: entry.into(),
        }),
    )
    .map_err(|e| e.to_string())
}
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub ss: u32,
    pub buffers: [Vec<u8>; 7],
}
impl FrameData {
    pub fn buffer_size(&self, index: usize) -> usize {
        match index {
            1 => (self.stride * self.ss * self.height * self.ss * 4) as usize,
            5 => (self.stride * self.height * 4) as usize,
            _ => self.buffers[index].len(),
        }
    }
    pub fn new(
        width: u32,
        ss: u32,
        quads: &[GpuQuad],
        bg: &[u32],
        fg: &[u32],
        smooth: bool,
        stretch: bool,
    ) -> Self {
        let stride = width.div_ceil(64) * 64;
        let (ranges, indices) = gpu_quad_bins(quads, width as usize);
        let mut bins = Vec::with_capacity(ranges.len() * 2 + indices.len());
        for range in ranges {
            bins.extend(range)
        }
        bins.extend(indices);
        let params = [
            quads.len() as u32,
            width,
            384,
            stride * ss,
            stride,
            1,
            ss,
            u32::from(smooth) | (u32::from(stretch) << 1),
            width.div_ceil(16),
            496,
            0,
            0,
        ];
        let bytes = |slice: &[u8]| {
            if slice.is_empty() {
                vec![0; 64]
            } else {
                slice.to_vec()
            }
        };
        Self {
            width,
            height: 384,
            stride,
            ss,
            buffers: [
                bytes(bytemuck::cast_slice(quads)),
                Vec::new(),
                bytemuck::cast_slice(&params).to_vec(),
                bytes(bytemuck::cast_slice(&bins)),
                bytes(bytemuck::cast_slice(bg)),
                Vec::new(),
                bytes(bytemuck::cast_slice(fg)),
            ],
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn wide_supersampling_preserves_standalone_uniforms_and_buffer_bounds() {
        let frame = super::FrameData::new(
            683,
            4,
            &[],
            &vec![0; 496 * 384],
            &vec![0; 496 * 384],
            true,
            true,
        );
        let params: Vec<u32> = frame.buffers[2]
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(&params[1..9], &[683, 384, 2816, 704, 1, 4, 3, 43]);
        assert_eq!(params[9], 496);
        assert_eq!(frame.buffer_size(1), 2816 * 1536 * 4);
        assert_eq!(frame.buffer_size(5), 704 * 384 * 4);
        assert!(frame.buffers[1].is_empty() && frame.buffers[5].is_empty());
    }

    #[test]
    fn shared_standalone_shaders_validate_and_compile() {
        assert!(!super::spirv(super::RASTER, "main").unwrap().is_empty());
        assert!(!super::spirv(super::RESOLVE, "resolve").unwrap().is_empty());
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Backend {
    #[default]
    Auto,
    Vulkan,
    OpenGl,
    Software,
}
#[derive(Clone, Copy)]
pub struct Settings {
    pub backend: Backend,
    pub wide: u32,
    pub ss: u32,
    pub srgb: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            backend: Backend::Auto,
            wide: 0,
            ss: 1,
            srgb: false,
        }
    }
}

pub enum Renderer {
    Vulkan(vulkan::Renderer),
    OpenGl(opengl::Renderer),
}
impl Renderer {
    // A reset without destroy means old API handles are no longer valid.
    // Drop host metadata but never call the dead device/context.
    pub fn abandon(self) {
        match self {
            Self::Vulkan(r) => r.abandon(),
            Self::OpenGl(r) => r.abandon(),
        }
    }
    pub unsafe fn render(&mut self, frame: &FrameData, srgb: bool) -> Result<(), String> {
        match self {
            Self::Vulkan(renderer) => renderer.render(frame, srgb),
            Self::OpenGl(renderer) => renderer.render(frame, srgb),
        }
    }
}
