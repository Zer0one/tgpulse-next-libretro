//! Vulkan compute wrapper; instance, device and queue are borrowed from Libretro.
use super::{spirv, FrameData, RASTER, RESOLVE};
use ash::{vk, Device, Instance};
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Image {
    pub view: vk::ImageView,
    pub layout: vk::ImageLayout,
    pub info: vk::ImageViewCreateInfo,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Interface {
    pub kind: u32,
    pub version: u32,
    pub handle: *mut c_void,
    pub instance: vk::Instance,
    pub gpu: vk::PhysicalDevice,
    pub device: vk::Device,
    pub get_device_proc: vk::PFN_vkGetDeviceProcAddr,
    pub get_instance_proc: vk::PFN_vkGetInstanceProcAddr,
    pub queue: vk::Queue,
    pub queue_index: u32,
    pub set_image: unsafe extern "C" fn(*mut c_void, *const Image, u32, *const vk::Semaphore, u32),
    pub get_sync_index: unsafe extern "C" fn(*mut c_void) -> u32,
    pub get_sync_mask: unsafe extern "C" fn(*mut c_void) -> u32,
    pub set_commands: unsafe extern "C" fn(*mut c_void, u32, *const vk::CommandBuffer),
    pub wait_sync: unsafe extern "C" fn(*mut c_void),
    pub lock_queue: unsafe extern "C" fn(*mut c_void),
    pub unlock_queue: unsafe extern "C" fn(*mut c_void),
    pub set_signal: Option<unsafe extern "C" fn(*mut c_void, vk::Semaphore)>,
}
unsafe impl Send for Interface {}
struct Buffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: usize,
}
struct Slot {
    pool: vk::CommandPool,
    cmd: vk::CommandBuffer,
    descriptors: vk::DescriptorPool,
    buffers: Vec<Buffer>,
    image: vk::Image,
    memory: vk::DeviceMemory,
    present: Image,
    width: u32,
    height: u32,
    srgb: bool,
}
pub struct Renderer {
    interface: Interface,
    device: Device,
    memory: vk::PhysicalDeviceMemoryProperties,
    layouts: [vk::DescriptorSetLayout; 2],
    pipeline_layouts: [vk::PipelineLayout; 2],
    pipelines: [vk::Pipeline; 2],
    slots: Vec<Option<Slot>>,
    context_valid: bool,
}
unsafe impl Send for Renderer {}
fn error(e: vk::Result) -> String {
    format!("Vulkan: {e:?}")
}
impl Renderer {
    pub fn abandon(mut self) {
        self.context_valid = false;
    }
    pub unsafe fn new(interface: Interface) -> Result<Self, String> {
        if interface.kind != 0 || interface.version < 5 {
            return Err("Libretro Vulkan interface v5 required".into());
        }
        let instance = Instance::load(
            &vk::StaticFn {
                get_instance_proc_addr: interface.get_instance_proc,
            },
            interface.instance,
        );
        let device = Device::load(instance.fp_v1_0(), interface.device);
        let memory = instance.get_physical_device_memory_properties(interface.gpu);
        let properties = instance.get_physical_device_queue_family_properties(interface.gpu);
        if !properties
            .get(interface.queue_index as usize)
            .is_some_and(|p| p.queue_flags.contains(vk::QueueFlags::COMPUTE))
        {
            return Err("Frontend queue lacks compute support".into());
        }
        let mut renderer = Self {
            interface,
            device,
            memory,
            layouts: [vk::DescriptorSetLayout::null(); 2],
            pipeline_layouts: [vk::PipelineLayout::null(); 2],
            pipelines: [vk::Pipeline::null(); 2],
            slots: (0..32).map(|_| None).collect(),
            context_valid: true,
        };
        for (pass, source, entry, count) in [(0, RASTER, "main", 5), (1, RESOLVE, "resolve", 4)] {
            let bindings: Vec<_> = (0..count)
                .map(|binding| {
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(binding)
                        .descriptor_count(1)
                        .descriptor_type(if binding == 2 {
                            vk::DescriptorType::UNIFORM_BUFFER
                        } else {
                            vk::DescriptorType::STORAGE_BUFFER
                        })
                        .stage_flags(vk::ShaderStageFlags::COMPUTE)
                        .build()
                })
                .collect();
            renderer.layouts[pass] = renderer
                .device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings),
                    None,
                )
                .map_err(error)?;
            renderer.pipeline_layouts[pass] = renderer
                .device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::builder().set_layouts(&[renderer.layouts[pass]]),
                    None,
                )
                .map_err(error)?;
            let module = renderer
                .device
                .create_shader_module(
                    &vk::ShaderModuleCreateInfo::builder().code(&spirv(source, entry)?),
                    None,
                )
                .map_err(error)?;
            let name = std::ffi::CString::new(entry).unwrap();
            let pipeline = renderer.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[vk::ComputePipelineCreateInfo::builder()
                    .stage(
                        vk::PipelineShaderStageCreateInfo::builder()
                            .stage(vk::ShaderStageFlags::COMPUTE)
                            .module(module)
                            .name(&name)
                            .build(),
                    )
                    .layout(renderer.pipeline_layouts[pass])
                    .build()],
                None,
            );
            renderer.device.destroy_shader_module(module, None);
            renderer.pipelines[pass] = pipeline.map_err(|(_, e)| error(e))?[0];
        }
        Ok(renderer)
    }
    fn memory_type(&self, bits: u32, flags: vk::MemoryPropertyFlags) -> Result<u32, String> {
        (0..self.memory.memory_type_count)
            .find(|&i| {
                bits & (1 << i) != 0
                    && self.memory.memory_types[i as usize]
                        .property_flags
                        .contains(flags)
            })
            .ok_or_else(|| format!("Vulkan memory type unavailable: {}", flags.as_raw()))
    }
    unsafe fn buffer(&self, size: usize) -> Result<Buffer, String> {
        let buffer = self
            .device
            .create_buffer(
                &vk::BufferCreateInfo::builder().size(size as u64).usage(
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::UNIFORM_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_SRC,
                ),
                None,
            )
            .map_err(error)?;
        let req = self.device.get_buffer_memory_requirements(buffer);
        let allocation = (|| {
            let memory = self
                .device
                .allocate_memory(
                    &vk::MemoryAllocateInfo::builder()
                        .allocation_size(req.size)
                        .memory_type_index(self.memory_type(
                            req.memory_type_bits,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        )?),
                    None,
                )
                .map_err(error)?;
            if let Err(e) = self.device.bind_buffer_memory(buffer, memory, 0) {
                self.device.free_memory(memory, None);
                return Err(error(e));
            }
            Ok(memory)
        })();
        match allocation {
            Ok(memory) => Ok(Buffer {
                buffer,
                memory,
                size,
            }),
            Err(e) => {
                self.device.destroy_buffer(buffer, None);
                Err(e)
            }
        }
    }
    unsafe fn slot(&self, width: u32, height: u32, srgb: bool) -> Result<Slot, String> {
        let pool = self
            .device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(self.interface.queue_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
            .map_err(error)?;
        let mut slot = Slot {
            pool,
            cmd: vk::CommandBuffer::null(),
            descriptors: vk::DescriptorPool::null(),
            buffers: Vec::new(),
            image: vk::Image::null(),
            memory: vk::DeviceMemory::null(),
            present: Image {
                view: vk::ImageView::null(),
                layout: vk::ImageLayout::GENERAL,
                info: vk::ImageViewCreateInfo::default(),
            },
            width,
            height,
            srgb,
        };
        let result = (|| {
            slot.cmd = self
                .device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_pool(pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )
                .map_err(error)?[0];
            slot.descriptors = self
                .device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::builder()
                        .max_sets(2)
                        .pool_sizes(&[
                            vk::DescriptorPoolSize {
                                ty: vk::DescriptorType::STORAGE_BUFFER,
                                descriptor_count: 7,
                            },
                            vk::DescriptorPoolSize {
                                ty: vk::DescriptorType::UNIFORM_BUFFER,
                                descriptor_count: 2,
                            },
                        ]),
                    None,
                )
                .map_err(error)?;
            slot.image = self
                .device
                .create_image(
                    &vk::ImageCreateInfo::builder()
                        .flags(vk::ImageCreateFlags::MUTABLE_FORMAT)
                        .image_type(vk::ImageType::TYPE_2D)
                        .format(vk::Format::B8G8R8A8_UNORM)
                        .extent(vk::Extent3D {
                            width,
                            height,
                            depth: 1,
                        })
                        .mip_levels(1)
                        .array_layers(1)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .usage(
                            vk::ImageUsageFlags::TRANSFER_DST
                                | vk::ImageUsageFlags::TRANSFER_SRC
                                | vk::ImageUsageFlags::SAMPLED,
                        ),
                    None,
                )
                .map_err(error)?;
            let req = self.device.get_image_memory_requirements(slot.image);
            slot.memory = self
                .device
                .allocate_memory(
                    &vk::MemoryAllocateInfo::builder()
                        .allocation_size(req.size)
                        .memory_type_index(self.memory_type(
                            req.memory_type_bits,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        )?),
                    None,
                )
                .map_err(error)?;
            self.device
                .bind_image_memory(slot.image, slot.memory, 0)
                .map_err(error)?;
            slot.present.info = vk::ImageViewCreateInfo::builder()
                .image(slot.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(if srgb {
                    vk::Format::B8G8R8A8_SRGB
                } else {
                    vk::Format::B8G8R8A8_UNORM
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build();
            slot.present.view = self
                .device
                .create_image_view(&slot.present.info, None)
                .map_err(error)?;
            Ok(())
        })();
        if let Err(e) = result {
            self.free_slot(slot);
            return Err(e);
        }
        Ok(slot)
    }
    unsafe fn free_buffer(&self, buffer: Buffer) {
        self.device.destroy_buffer(buffer.buffer, None);
        self.device.free_memory(buffer.memory, None)
    }
    unsafe fn free_slot(&self, slot: Slot) {
        for buffer in slot.buffers {
            self.free_buffer(buffer)
        }
        self.device.destroy_image_view(slot.present.view, None);
        self.device.destroy_image(slot.image, None);
        self.device.free_memory(slot.memory, None);
        self.device.destroy_descriptor_pool(slot.descriptors, None);
        self.device.destroy_command_pool(slot.pool, None);
    }
    pub unsafe fn render(&mut self, frame: &FrameData, srgb: bool) -> Result<(), String> {
        let interface = self.interface;
        let index = (interface.get_sync_index)(interface.handle) as usize;
        let mask = (interface.get_sync_mask)(interface.handle);
        if index >= 32 || mask & (1 << index) == 0 {
            return Err("Invalid frontend Vulkan sync index".into());
        }
        (interface.wait_sync)(interface.handle);
        let resize = self.slots[index]
            .as_ref()
            .is_some_and(|s| s.width != frame.width || s.height != frame.height || s.srgb != srgb);
        if resize {
            if let Some(old) = self.slots[index].take() {
                self.free_slot(old)
            }
        }
        if self.slots[index].is_none() {
            self.slots[index] = Some(self.slot(frame.width, frame.height, srgb)?)
        }
        let mut slot = self.slots[index].take().unwrap();
        let result = self.record(&mut slot, frame);
        self.slots[index] = Some(slot);
        result?;
        let slot = self.slots[index].as_ref().unwrap();
        (interface.set_commands)(interface.handle, 1, &slot.cmd);
        (interface.set_image)(
            interface.handle,
            &slot.present,
            0,
            ptr::null(),
            interface.queue_index,
        );
        Ok(())
    }
    unsafe fn record(&self, slot: &mut Slot, frame: &FrameData) -> Result<(), String> {
        self.device
            .reset_command_pool(slot.pool, vk::CommandPoolResetFlags::empty())
            .map_err(error)?;
        self.device
            .reset_descriptor_pool(slot.descriptors, vk::DescriptorPoolResetFlags::empty())
            .map_err(error)?;
        for (i, data) in frame.buffers.iter().enumerate() {
            if slot.buffers.len() <= i {
                slot.buffers.push(self.buffer(frame.buffer_size(i))?)
            } else if slot.buffers[i].size < frame.buffer_size(i) {
                let new = self.buffer(frame.buffer_size(i))?;
                let old = std::mem::replace(&mut slot.buffers[i], new);
                self.free_buffer(old)
            }
            if i != 1 && i != 5 {
                let mapped = self
                    .device
                    .map_memory(
                        slot.buffers[i].memory,
                        0,
                        data.len() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                    .map_err(error)?;
                ptr::copy_nonoverlapping(data.as_ptr(), mapped.cast(), data.len());
                self.device.unmap_memory(slot.buffers[i].memory)
            }
        }
        let sets = self
            .device
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(slot.descriptors)
                    .set_layouts(&self.layouts),
            )
            .map_err(error)?;
        for (pass, indices) in [(0, &[0, 1, 2, 3, 4][..]), (1, &[1, 5, 2, 6][..])] {
            let infos: Vec<_> = indices
                .iter()
                .map(|&i| vk::DescriptorBufferInfo {
                    buffer: slot.buffers[i].buffer,
                    offset: 0,
                    range: frame.buffer_size(i) as u64,
                })
                .collect();
            let writes: Vec<_> = infos
                .iter()
                .enumerate()
                .map(|(binding, info)| {
                    vk::WriteDescriptorSet::builder()
                        .dst_set(sets[pass])
                        .dst_binding(binding as u32)
                        .descriptor_type(if binding == 2 {
                            vk::DescriptorType::UNIFORM_BUFFER
                        } else {
                            vk::DescriptorType::STORAGE_BUFFER
                        })
                        .buffer_info(std::slice::from_ref(info))
                        .build()
                })
                .collect();
            self.device.update_descriptor_sets(&writes, &[]);
        }
        let cmd = slot.cmd;
        self.device
            .begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .map_err(error)?;
        self.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::HOST_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build()],
            &[],
            &[],
        );
        for pass in 0..2 {
            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipelines[pass],
            );
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layouts[pass],
                0,
                &[sets[pass]],
                &[],
            );
            let scale = if pass == 0 { frame.ss } else { 1 };
            self.device.cmd_dispatch(
                cmd,
                (frame.width * scale).div_ceil(8),
                (frame.height * scale).div_ceil(8),
                1,
            );
            self.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                if pass == 0 {
                    vk::PipelineStageFlags::COMPUTE_SHADER
                } else {
                    vk::PipelineStageFlags::TRANSFER
                },
                vk::DependencyFlags::empty(),
                &[vk::MemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                    .dst_access_mask(if pass == 0 {
                        vk::AccessFlags::SHADER_READ
                    } else {
                        vk::AccessFlags::TRANSFER_READ
                    })
                    .build()],
                &[],
                &[],
            );
        }
        let range = slot.present.info.subresource_range;
        self.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .image(slot.image)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(range)
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .build()],
        );
        self.device.cmd_copy_buffer_to_image(
            cmd,
            slot.buffers[5].buffer,
            slot.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy::builder()
                .buffer_row_length(frame.stride)
                .buffer_image_height(frame.height)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D {
                    width: frame.width,
                    height: frame.height,
                    depth: 1,
                })
                .build()],
        );
        self.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .image(slot.image)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(range)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build()],
        );
        self.device.end_command_buffer(cmd).map_err(error)
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        if !self.context_valid {
            return;
        }
        unsafe {
            (self.interface.lock_queue)(self.interface.handle);
            let _ = self.device.queue_wait_idle(self.interface.queue);
            (self.interface.unlock_queue)(self.interface.handle);
            for i in 0..self.slots.len() {
                if let Some(slot) = self.slots[i].take() {
                    self.free_slot(slot)
                }
            }
            for i in 0..2 {
                self.device.destroy_pipeline(self.pipelines[i], None);
                self.device
                    .destroy_pipeline_layout(self.pipeline_layouts[i], None);
                self.device
                    .destroy_descriptor_set_layout(self.layouts[i], None)
            }
        }
    }
}
