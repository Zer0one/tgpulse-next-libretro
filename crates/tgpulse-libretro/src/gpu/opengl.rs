//! OpenGL 4.3/GLES 3.1 compute wrapper; context and target FBO belong to frontend.
use super::{shader, FrameData, HwCallback, RASTER, RESOLVE};
use glow::HasContext;
use naga::back::glsl;
pub fn source(source: &str, entry: &str, es: bool) -> Result<String, String> {
    let (module, info) = shader(source)?;
    let mut output = String::new();
    let options = glsl::Options {
        version: if es {
            glsl::Version::new_gles(310)
        } else {
            glsl::Version::Desktop(430)
        },
        binding_map: (0..5)
            .map(|b| {
                (
                    naga::ResourceBinding {
                        group: 0,
                        binding: b,
                    },
                    b as u8,
                )
            })
            .collect(),
        ..Default::default()
    };
    let pipeline = glsl::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: entry.into(),
        multiview: None,
    };
    glsl::Writer::new(
        &mut output,
        &module,
        &info,
        &options,
        &pipeline,
        naga::proc::BoundsCheckPolicies::default(),
    )
    .map_err(|e| e.to_string())?
    .write()
    .map_err(|e| e.to_string())?;
    Ok(output)
}
pub struct Renderer {
    gl: glow::Context,
    callback: HwCallback,
    programs: [glow::NativeProgram; 3],
    buffers: Vec<glow::NativeBuffer>,
    texture: glow::NativeTexture,
    vao: glow::NativeVertexArray,
    width: u32,
    height: u32,
    srgb: bool,
    es: bool,
    context_valid: bool,
}
// Access is serialized by the core mutex and callbacks execute on frontend GL thread.
unsafe impl Send for Renderer {}
unsafe fn program(
    gl: &glow::Context,
    sources: &[(u32, String)],
) -> Result<glow::NativeProgram, String> {
    let program = gl.create_program()?;
    let mut shaders = Vec::new();
    let result = (|| {
        for (kind, source) in sources {
            let shader = gl.create_shader(*kind)?;
            shaders.push(shader);
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(gl.get_program_info_log(program));
        }
        Ok(())
    })();
    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader)
    }
    if let Err(error) = result {
        gl.delete_program(program);
        return Err(error);
    }
    Ok(program)
}
impl Renderer {
    pub fn abandon(mut self) {
        self.context_valid = false;
    }
    pub unsafe fn new(callback: HwCallback) -> Result<Self, String> {
        let load = callback
            .get_proc_address
            .ok_or("Frontend GL symbol loader missing")?;
        let gl = glow::Context::from_loader_function(|name| {
            let name = std::ffi::CString::new(name).unwrap();
            load(name.as_ptr())
        });
        let version = gl.version();
        let es = version.is_embedded;
        if (version.major, version.minor) < if es { (3, 1) } else { (4, 3) } {
            return Err(format!(
                "OpenGL compute requires 4.3 or GLES 3.1; received {version:?}"
            ));
        }
        let prefix = if es {
            "#version 310 es\nprecision highp float;\nprecision highp int;\n"
        } else {
            "#version 430 core\n"
        };
        let vertex=format!("{prefix}out vec2 uv;void main(){{vec2 p=vec2(float((gl_VertexID<<1)&2),float(gl_VertexID&2));uv=vec2(p.x,1.0-p.y);gl_Position=vec4(p*2.0-1.0,0.0,1.0);}}");
        let fragment=format!("{prefix}in vec2 uv;uniform sampler2D image;out vec4 color;void main(){{color=vec4(texture(image,uv).bgr,1.0);}}");
        let raster = program(&gl, &[(glow::COMPUTE_SHADER, source(RASTER, "main", es)?)])?;
        let resolve = match program(
            &gl,
            &[(glow::COMPUTE_SHADER, source(RESOLVE, "resolve", es)?)],
        ) {
            Ok(p) => p,
            Err(e) => {
                gl.delete_program(raster);
                return Err(e);
            }
        };
        let present = match program(
            &gl,
            &[
                (glow::VERTEX_SHADER, vertex),
                (glow::FRAGMENT_SHADER, fragment),
            ],
        ) {
            Ok(p) => p,
            Err(e) => {
                gl.delete_program(raster);
                gl.delete_program(resolve);
                return Err(e);
            }
        };
        let texture = gl.create_texture()?;
        let vao = gl.create_vertex_array()?;
        let mut buffers = Vec::new();
        for _ in 0..7 {
            buffers.push(gl.create_buffer()?)
        }
        Ok(Self {
            gl,
            callback,
            programs: [raster, resolve, present],
            buffers,
            texture,
            vao,
            width: 0,
            height: 0,
            srgb: false,
            es,
            context_valid: true,
        })
    }
    pub unsafe fn render(&mut self, frame: &FrameData, srgb: bool) -> Result<(), String> {
        let gl = &self.gl;
        // GL buffer replacement keeps pending GPU reads alive; no CPU readback.
        for (i, data) in frame.buffers.iter().enumerate() {
            let target = if i == 2 {
                glow::UNIFORM_BUFFER
            } else {
                glow::SHADER_STORAGE_BUFFER
            };
            gl.bind_buffer(target, Some(self.buffers[i]));
            if i == 1 || i == 5 {
                gl.buffer_data_size(target, frame.buffer_size(i) as i32, glow::DYNAMIC_DRAW)
            } else {
                gl.buffer_data_u8_slice(target, data, glow::DYNAMIC_DRAW)
            }
        }
        for (pass, indices) in [(0, &[0, 1, 2, 3, 4][..]), (1, &[1, 5, 2, 6][..])] {
            gl.use_program(Some(self.programs[pass]));
            for (binding, &index) in indices.iter().enumerate() {
                gl.bind_buffer_base(
                    if binding == 2 {
                        glow::UNIFORM_BUFFER
                    } else {
                        glow::SHADER_STORAGE_BUFFER
                    },
                    binding as u32,
                    Some(self.buffers[index]),
                );
            }
            let scale = if pass == 0 { frame.ss } else { 1 };
            gl.dispatch_compute(
                (frame.width * scale).div_ceil(8),
                (frame.height * scale).div_ceil(8),
                1,
            );
            gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT | glow::PIXEL_BUFFER_BARRIER_BIT);
        }
        gl.active_texture(glow::TEXTURE0);
        gl.bind_sampler(0, None);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(self.buffers[5]));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, frame.stride as i32);
        if self.width != frame.width || self.height != frame.height || self.srgb != srgb {
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                if srgb {
                    glow::SRGB8_ALPHA8
                } else {
                    glow::RGBA8
                } as i32,
                frame.width as i32,
                frame.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            self.width = frame.width;
            self.height = frame.height;
            self.srgb = srgb;
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(self.buffers[5]));
        }
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            frame.width as i32,
            frame.height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::BufferOffset(0),
        );
        gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
        let fbo = self
            .callback
            .get_current_framebuffer
            .ok_or("Frontend framebuffer callback missing")?();
        gl.bind_framebuffer(
            glow::FRAMEBUFFER,
            std::num::NonZeroU32::new(fbo as u32).map(glow::NativeFramebuffer),
        );
        gl.viewport(0, 0, frame.width as i32, frame.height as i32);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::STENCIL_TEST);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::BLEND);
        gl.color_mask(true, true, true, true);
        if !self.es {
            gl.enable(glow::FRAMEBUFFER_SRGB)
        }
        gl.use_program(Some(self.programs[2]));
        gl.uniform_1_i32(
            gl.get_uniform_location(self.programs[2], "image").as_ref(),
            0,
        );
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
        gl.use_program(None);
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            return Err(format!("OpenGL frame error: 0x{error:x}"));
        }
        Ok(())
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        if !self.context_valid {
            return;
        }
        unsafe {
            self.gl.finish();
            for program in self.programs {
                self.gl.delete_program(program)
            }
            for buffer in &self.buffers {
                self.gl.delete_buffer(*buffer)
            }
            self.gl.delete_texture(self.texture);
            self.gl.delete_vertex_array(self.vao)
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn standalone_shaders_translate_to_desktop_and_es() {
        for es in [false, true] {
            assert!(super::source(super::RASTER, "main", es)
                .unwrap()
                .contains("void main"));
            assert!(super::source(super::RESOLVE, "resolve", es)
                .unwrap()
                .contains("void main"));
        }
    }
}
