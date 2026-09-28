use super::*;

#[test]
fn framebuffer_decode_only_compensates_srgb_surfaces() {
    use wgpu::TextureFormat::*;
    for surface in [Bgra8Unorm, Rgba8Unorm] {
        assert_eq!(framebuffer_format(true, surface), Bgra8Unorm);
        assert_eq!(framebuffer_format(false, surface), Bgra8Unorm);
    }
    for surface in [Bgra8UnormSrgb, Rgba8UnormSrgb] {
        assert_eq!(framebuffer_format(true, surface), Bgra8UnormSrgb);
        assert_eq!(framebuffer_format(false, surface), Bgra8Unorm);
    }
}

/// Real shader/readback regression; no window, ROM or user settings required.
/// A machine without a GPU can still run the pure format and core tests.
#[test]
fn blit_pixel_ramp_preserves_display_bytes_when_corrected() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let Some(adapter) = instance.request_adapter(&Default::default()).await else {
            eprintln!("GPU ramp not exercised: no wgpu adapter available");
            return;
        };
        eprintln!("GPU ramp adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&Default::default());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let size = wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        };
        let pixels: Vec<u8> = (0..=255u8).flat_map(|v| [v, v, v, 255]).collect();
        for surface in [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            for enabled in [false, true, false] {
                let (source, bind_group) = Model2Video::make_fb_texture(
                    &device,
                    &bgl,
                    &sampler,
                    256,
                    1,
                    framebuffer_format(enabled, surface),
                );
                queue.write_texture(
                    source.as_image_copy(),
                    &pixels,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(1024),
                        rows_per_image: Some(1),
                    },
                    size,
                );
                let target = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: surface,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(surface.into())],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview: None,
                });
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 1024,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let view = target.create_view(&Default::default());
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.draw(0..3, 0..1);
                }
                encoder.copy_texture_to_buffer(
                    target.as_image_copy(),
                    wgpu::ImageCopyBuffer {
                        buffer: &buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(1024),
                            rows_per_image: Some(1),
                        },
                    },
                    size,
                );
                queue.submit(Some(encoder.finish()));
                let (tx, rx) = std::sync::mpsc::channel();
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
                device.poll(wgpu::Maintain::Wait);
                rx.recv().unwrap().unwrap();
                let read = buffer.slice(..).get_mapped_range();
                for v in 0..=255usize {
                    let expected = if surface.is_srgb() && !enabled {
                        let linear = v as f64 / 255.0;
                        let encoded = if linear <= 0.0031308 {
                            linear * 12.92
                        } else {
                            1.055 * linear.powf(1.0 / 2.4) - 0.055
                        };
                        (encoded * 255.0).round() as i32
                    } else {
                        v as i32
                    };
                    for channel in 0..3 {
                        assert!(
                            (read[v * 4 + channel] as i32 - expected).abs() <= 1,
                            "{surface:?} enabled={enabled} input={v} actual={}",
                            read[v * 4 + channel]
                        );
                    }
                    assert_eq!(read[v * 4 + 3], 255);
                }
                drop(read);
                buffer.unmap();
            }
        }
    });
}
