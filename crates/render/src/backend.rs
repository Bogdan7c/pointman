use crate::mesh::Vertex;
use crate::{DrawList, MeshId, RenderError};
use ash::khr::{surface as khr_surface, swapchain as khr_swapchain};
use ash::{vk, Device, Entry, Instance};
use bytemuck::{Pod, Zeroable};
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator, AllocatorCreateDesc};
use gpu_allocator::MemoryLocation;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, CString};
use std::mem::size_of;
use winit::window::Window;

const FRAMES: usize = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUbo {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    pos_radius: [[f32; 4]; 8],
    color_intensity: [[f32; 4]; 8],
    light_count: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Push {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

struct GpuBuffer {
    buffer: vk::Buffer,
    alloc: Option<Allocation>,
}

struct GpuMesh {
    vb: GpuBuffer,
    ib: GpuBuffer,
    index_count: u32,
}

struct GpuImage {
    image: vk::Image,
    view: vk::ImageView,
    alloc: Allocation,
}

struct FrameSync {
    cmd: vk::CommandBuffer,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    ubo: GpuBuffer,
    ubo_set: vk::DescriptorSet,
}

pub struct Renderer {
    _entry: Entry,
    instance: Instance,
    surface_loader: khr_surface::Instance,
    surface: vk::SurfaceKHR,
    physical: vk::PhysicalDevice,
    device: Device,
    queue: vk::Queue,
    queue_family: u32,
    allocator: Option<Allocator>,
    swapchain_loader: khr_swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swap_images: Vec<vk::Image>,
    swap_views: Vec<vk::ImageView>,
    swap_format: vk::Format,
    extent: vk::Extent2D,
    gbuffer_pass: vk::RenderPass,
    lighting_pass: vk::RenderPass,
    albedo: Option<GpuImage>,
    normal: Option<GpuImage>,
    depth: Option<GpuImage>,
    gbuffer_fb: vk::Framebuffer,
    light_fbs: Vec<vk::Framebuffer>,
    sampler: vk::Sampler,
    ubo_layout: vk::DescriptorSetLayout,
    sampled_layout: vk::DescriptorSetLayout,
    gbuffer_pipe_layout: vk::PipelineLayout,
    lighting_pipe_layout: vk::PipelineLayout,
    gbuffer_pipe: vk::Pipeline,
    lighting_pipe: vk::Pipeline,
    descriptor_pool: vk::DescriptorPool,
    sampled_set: vk::DescriptorSet,
    cmd_pool: vk::CommandPool,
    frames: Vec<FrameSync>,
    frame_index: usize,
    meshes: Vec<GpuMesh>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self, RenderError> {
        let entry = unsafe { Entry::load()? };
        let display = window.display_handle().map_err(|_| RenderError::Window)?.as_raw();
        let win = window.window_handle().map_err(|_| RenderError::Window)?.as_raw();

        let app_name = CString::new("pointman").unwrap();
        let engine_name = CString::new("pointman").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_1);

        let ext_ptrs = ash_window::enumerate_required_extensions(display)?;
        let mut instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(ext_ptrs);

        let validation = CString::new("VK_LAYER_KHRONOS_validation").unwrap();
        let layer_ptr = [validation.as_ptr()];
        let has_validation = unsafe { entry.enumerate_instance_layer_properties()? }
            .iter()
            .any(|l| {
                let n = unsafe { CStr::from_ptr(l.layer_name.as_ptr()) };
                n.to_bytes() == b"VK_LAYER_KHRONOS_validation"
            });
        if has_validation {
            instance_info = instance_info.enabled_layer_names(&layer_ptr);
        }

        let instance = unsafe { entry.create_instance(&instance_info, None)? };
        let surface_loader = khr_surface::Instance::new(&entry, &instance);
        let surface = unsafe { ash_window::create_surface(&entry, &instance, display, win, None)? };

        let (physical, queue_family) = pick_gpu(&instance, &surface_loader, surface)?;
        let priorities = [1.0f32];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities);
        let device_exts = [khr_swapchain::NAME.as_ptr()];
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_exts);
        let device = unsafe { instance.create_device(physical, &device_info, None)? };
        let queue = unsafe { device.get_device_queue(queue_family, 0) };
        let swapchain_loader = khr_swapchain::Device::new(&instance, &device);

        let mut allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: physical,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })?;

        let size = window.inner_size();
        let (swapchain, swap_images, swap_views, swap_format, extent) = create_swapchain(
            &instance,
            &device,
            &surface_loader,
            &swapchain_loader,
            physical,
            surface,
            queue_family,
            vk::SwapchainKHR::null(),
            vk::Extent2D {
                width: size.width.max(1),
                height: size.height.max(1),
            },
        )?;

        let gbuffer_pass = create_gbuffer_pass(&device)?;
        let lighting_pass = create_lighting_pass(&device, swap_format)?;
        let sampler = create_sampler(&device)?;
        let (ubo_layout, sampled_layout) = create_set_layouts(&device)?;
        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(size_of::<Push>() as u32);
        let gbuffer_set_layouts = [ubo_layout];
        let gbuffer_pipe_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&gbuffer_set_layouts)
                    .push_constant_ranges(std::slice::from_ref(&push_range)),
                None,
            )?
        };
        let lighting_set_layouts = [ubo_layout, sampled_layout];
        let lighting_pipe_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().set_layouts(&lighting_set_layouts),
                None,
            )?
        };

        let gbuffer_vert = load_shader(&device, include_bytes!(concat!(env!("OUT_DIR"), "/gbuffer.vert.spv")))?;
        let gbuffer_frag = load_shader(&device, include_bytes!(concat!(env!("OUT_DIR"), "/gbuffer.frag.spv")))?;
        let lighting_vert = load_shader(&device, include_bytes!(concat!(env!("OUT_DIR"), "/lighting.vert.spv")))?;
        let lighting_frag = load_shader(&device, include_bytes!(concat!(env!("OUT_DIR"), "/lighting.frag.spv")))?;
        let gbuffer_pipe = create_gbuffer_pipeline(
            &device,
            gbuffer_pass,
            gbuffer_pipe_layout,
            gbuffer_vert,
            gbuffer_frag,
        )?;
        let lighting_pipe = create_lighting_pipeline(
            &device,
            lighting_pass,
            lighting_pipe_layout,
            lighting_vert,
            lighting_frag,
        )?;
        unsafe {
            device.destroy_shader_module(gbuffer_vert, None);
            device.destroy_shader_module(gbuffer_frag, None);
            device.destroy_shader_module(lighting_vert, None);
            device.destroy_shader_module(lighting_frag, None);
        }

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: FRAMES as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 3,
            },
        ];
        let descriptor_pool = unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets((FRAMES + 1) as u32)
                    .pool_sizes(&pool_sizes),
                None,
            )?
        };

        let cmd_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )?
        };
        let cmds = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(cmd_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(FRAMES as u32),
            )?
        };

        let mut frames = Vec::new();
        for i in 0..FRAMES {
            let ubo = create_buffer(
                &device,
                &mut allocator,
                size_of::<FrameUbo>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                MemoryLocation::CpuToGpu,
                "frame-ubo",
            )?;
            let set_layouts = [ubo_layout];
            let ubo_set = unsafe {
                device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(&set_layouts),
                )?[0]
            };
            let buf_info = vk::DescriptorBufferInfo::default()
                .buffer(ubo.buffer)
                .offset(0)
                .range(size_of::<FrameUbo>() as u64);
            let write = vk::WriteDescriptorSet::default()
                .dst_set(ubo_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buf_info));
            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
            frames.push(FrameSync {
                cmd: cmds[i],
                image_available: unsafe {
                    device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
                },
                render_finished: unsafe {
                    device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
                },
                in_flight: unsafe {
                    device.create_fence(
                        &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                        None,
                    )?
                },
                ubo,
                ubo_set,
            });
        }

        let sampled_layouts = [sampled_layout];
        let sampled_set = unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(descriptor_pool)
                    .set_layouts(&sampled_layouts),
            )?[0]
        };

        let (verts, inds16) = crate::mesh::cube();
        let inds: Vec<u32> = inds16.iter().map(|&i| u32::from(i)).collect();
        let cube_vb = upload_buffer(
            &device,
            queue,
            cmd_pool,
            &mut allocator,
            bytemuck::cast_slice(&verts),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            "cube-vb",
        )?;
        let cube_ib = upload_buffer(
            &device,
            queue,
            cmd_pool,
            &mut allocator,
            bytemuck::cast_slice(&inds),
            vk::BufferUsageFlags::INDEX_BUFFER,
            "cube-ib",
        )?;

        let mut renderer = Self {
            _entry: entry,
            instance,
            surface_loader,
            surface,
            physical,
            device,
            queue,
            queue_family,
            allocator: Some(allocator),
            swapchain_loader,
            swapchain,
            swap_images,
            swap_views,
            swap_format,
            extent,
            gbuffer_pass,
            lighting_pass,
            albedo: None,
            normal: None,
            depth: None,
            gbuffer_fb: vk::Framebuffer::null(),
            light_fbs: Vec::new(),
            sampler,
            ubo_layout,
            sampled_layout,
            gbuffer_pipe_layout,
            lighting_pipe_layout,
            gbuffer_pipe,
            lighting_pipe,
            descriptor_pool,
            sampled_set,
            cmd_pool,
            frames,
            frame_index: 0,
            meshes: vec![GpuMesh {
                vb: cube_vb,
                ib: cube_ib,
                index_count: inds.len() as u32,
            }],
        };
        renderer.recreate_gbuffer()?;
        Ok(renderer)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.recreate_swapchain(vk::Extent2D { width, height })
    }

    pub fn upload_mesh(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> Result<MeshId, RenderError> {
        let allocator = self
            .allocator
            .as_mut()
            .ok_or_else(|| RenderError::Alloc("allocator gone".into()))?;
        let vb = upload_buffer(
            &self.device,
            self.queue,
            self.cmd_pool,
            allocator,
            bytemuck::cast_slice(vertices),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            "mesh-vb",
        )?;
        let ib = upload_buffer(
            &self.device,
            self.queue,
            self.cmd_pool,
            allocator,
            bytemuck::cast_slice(indices),
            vk::BufferUsageFlags::INDEX_BUFFER,
            "mesh-ib",
        )?;
        let id = MeshId(self.meshes.len() as u32);
        self.meshes.push(GpuMesh {
            vb,
            ib,
            index_count: indices.len() as u32,
        });
        Ok(id)
    }

    pub fn draw(&mut self, list: &DrawList) -> Result<(), RenderError> {
        if self.extent.width == 0 || self.extent.height == 0 {
            return Ok(());
        }
        let frame_i = self.frame_index;
        let fence = self.frames[frame_i].in_flight;
        unsafe {
            self.device.wait_for_fences(&[fence], true, u64::MAX)?;
        }

        let acquire = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.frames[frame_i].image_available,
                vk::Fence::null(),
            )
        };
        let image_index = match acquire {
            Ok((_idx, true)) => {
                self.recreate_swapchain(self.extent)?;
                return Ok(());
            }
            Ok((idx, false)) => idx,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(self.extent)?;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        unsafe { self.device.reset_fences(&[fence])? };
        self.update_ubo(frame_i, list)?;
        self.record(frame_i, image_index as usize, list)?;

        let wait = [self.frames[frame_i].image_available];
        let signal = [self.frames[frame_i].render_finished];
        let cmds = [self.frames[frame_i].cmd];
        let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&wait)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&cmds)
            .signal_semaphores(&signal);
        unsafe { self.device.queue_submit(self.queue, &[submit], fence)? };

        let swaps = [self.swapchain];
        let idx = [image_index];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal)
            .swapchains(&swaps)
            .image_indices(&idx);
        match unsafe { self.swapchain_loader.queue_present(self.queue, &present) } {
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(self.extent)?;
            }
            Ok(false) => {}
            Err(e) => return Err(e.into()),
        }
        self.frame_index = (self.frame_index + 1) % FRAMES;
        Ok(())
    }

    fn update_ubo(&mut self, frame_i: usize, list: &DrawList) -> Result<(), RenderError> {
        let aspect = self.extent.width as f32 / self.extent.height.max(1) as f32;
        let view_proj = list.camera.view_proj(aspect);
        let mut ubo = FrameUbo {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            camera_pos: [
                list.camera.position.x,
                list.camera.position.y,
                list.camera.position.z,
                1.0,
            ],
            pos_radius: [[0.0; 4]; 8],
            color_intensity: [[0.0; 4]; 8],
            light_count: list.lights.len().min(8) as u32,
            _pad: [0; 3],
        };
        for (i, light) in list.lights.iter().take(8).enumerate() {
            ubo.pos_radius[i] = [light.position.x, light.position.y, light.position.z, light.radius];
            ubo.color_intensity[i] = [light.color.x, light.color.y, light.color.z, light.intensity];
        }
        let alloc = self.frames[frame_i]
            .ubo
            .alloc
            .as_ref()
            .ok_or_else(|| RenderError::Alloc("ubo missing".into()))?;
        let ptr = alloc
            .mapped_ptr()
            .ok_or_else(|| RenderError::Alloc("ubo not mapped".into()))?;
        unsafe {
            ptr.cast::<FrameUbo>().as_ptr().write(ubo);
        }
        Ok(())
    }

    fn record(&self, frame_i: usize, image_index: usize, list: &DrawList) -> Result<(), RenderError> {
        let cmd = self.frames[frame_i].cmd;
        unsafe {
            self.device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
        }
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.extent.width as f32,
            height: self.extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.extent,
        };
        unsafe {
            self.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.device.cmd_set_scissor(cmd, 0, &[scissor]);
        }

        let clears = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.5, 0.5, 1.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let gbegin = vk::RenderPassBeginInfo::default()
            .render_pass(self.gbuffer_pass)
            .framebuffer(self.gbuffer_fb)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            })
            .clear_values(&clears);
        unsafe {
            self.device
                .cmd_begin_render_pass(cmd, &gbegin, vk::SubpassContents::INLINE);
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.gbuffer_pipe);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.gbuffer_pipe_layout,
                0,
                &[self.frames[frame_i].ubo_set],
                &[],
            );
            self.device
                .cmd_bind_vertex_buffers(cmd, 0, &[self.meshes[0].vb.buffer], &[0]);
            self.device.cmd_bind_index_buffer(
                cmd,
                self.meshes[0].ib.buffer,
                0,
                vk::IndexType::UINT32,
            );
        }
        let mut bound = 0u32;
        for inst in &list.instances {
            let mesh_id = inst.mesh.0 as usize;
            if mesh_id >= self.meshes.len() {
                continue;
            }
            if mesh_id as u32 != bound {
                bound = mesh_id as u32;
                unsafe {
                    self.device.cmd_bind_vertex_buffers(
                        cmd,
                        0,
                        &[self.meshes[mesh_id].vb.buffer],
                        &[0],
                    );
                    self.device.cmd_bind_index_buffer(
                        cmd,
                        self.meshes[mesh_id].ib.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                }
            }
            let index_count = if inst.index_count == 0 {
                self.meshes[mesh_id].index_count
            } else {
                inst.index_count
            };
            let push = Push {
                model: inst.transform.to_cols_array_2d(),
                color: inst.color.to_array(),
            };
            unsafe {
                self.device.cmd_push_constants(
                    cmd,
                    self.gbuffer_pipe_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytemuck::bytes_of(&push),
                );
                self.device.cmd_draw_indexed(cmd, index_count, 1, inst.first_index, 0, 0);
            }
        }
        unsafe { self.device.cmd_end_render_pass(cmd) };

        let lclears = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];
        let lbegin = vk::RenderPassBeginInfo::default()
            .render_pass(self.lighting_pass)
            .framebuffer(self.light_fbs[image_index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            })
            .clear_values(&lclears);
        unsafe {
            self.device
                .cmd_begin_render_pass(cmd, &lbegin, vk::SubpassContents::INLINE);
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.lighting_pipe);
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.lighting_pipe_layout,
                0,
                &[self.frames[frame_i].ubo_set, self.sampled_set],
                &[],
            );
            self.device.cmd_draw(cmd, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd)?;
        }
        Ok(())
    }

    fn recreate_swapchain(&mut self, extent: vk::Extent2D) -> Result<(), RenderError> {
        unsafe { self.device.device_wait_idle()? };
        self.destroy_swapchain_resources();
        let (swapchain, images, views, format, extent) = create_swapchain(
            &self.instance,
            &self.device,
            &self.surface_loader,
            &self.swapchain_loader,
            self.physical,
            self.surface,
            self.queue_family,
            self.swapchain,
            extent,
        )?;
        unsafe { self.swapchain_loader.destroy_swapchain(self.swapchain, None) };
        self.swapchain = swapchain;
        self.swap_images = images;
        self.swap_views = views;
        self.swap_format = format;
        self.extent = extent;
        self.recreate_gbuffer()
    }

    fn recreate_gbuffer(&mut self) -> Result<(), RenderError> {
        self.destroy_gbuffer();
        let allocator = self.allocator.as_mut().unwrap();
        let albedo = create_color_target(
            &self.device,
            allocator,
            self.extent,
            vk::Format::R8G8B8A8_UNORM,
            "albedo",
        )?;
        let normal = create_color_target(
            &self.device,
            allocator,
            self.extent,
            vk::Format::R8G8B8A8_UNORM,
            "normal",
        )?;
        let depth = create_depth_target(&self.device, allocator, self.extent)?;
        let attachments = [albedo.view, normal.view, depth.view];
        self.gbuffer_fb = unsafe {
            self.device.create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(self.gbuffer_pass)
                    .attachments(&attachments)
                    .width(self.extent.width)
                    .height(self.extent.height)
                    .layers(1),
                None,
            )?
        };
        self.light_fbs.clear();
        for view in &self.swap_views {
            let atts = [*view];
            self.light_fbs.push(unsafe {
                self.device.create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(self.lighting_pass)
                        .attachments(&atts)
                        .width(self.extent.width)
                        .height(self.extent.height)
                        .layers(1),
                    None,
                )?
            });
        }
        let infos = [
            vk::DescriptorImageInfo::default()
                .sampler(self.sampler)
                .image_view(albedo.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            vk::DescriptorImageInfo::default()
                .sampler(self.sampler)
                .image_view(normal.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
            vk::DescriptorImageInfo::default()
                .sampler(self.sampler)
                .image_view(depth.view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
        ];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(self.sampled_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[0])),
            vk::WriteDescriptorSet::default()
                .dst_set(self.sampled_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[1])),
            vk::WriteDescriptorSet::default()
                .dst_set(self.sampled_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&infos[2])),
        ];
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };
        self.albedo = Some(albedo);
        self.normal = Some(normal);
        self.depth = Some(depth);
        Ok(())
    }

    fn destroy_gbuffer(&mut self) {
        unsafe {
            if self.gbuffer_fb != vk::Framebuffer::null() {
                self.device.destroy_framebuffer(self.gbuffer_fb, None);
                self.gbuffer_fb = vk::Framebuffer::null();
            }
            for fb in self.light_fbs.drain(..) {
                self.device.destroy_framebuffer(fb, None);
            }
        }
        let allocator = self.allocator.as_mut().unwrap();
        for slot in [&mut self.albedo, &mut self.normal, &mut self.depth] {
            if let Some(img) = slot.take() {
                unsafe { self.device.destroy_image_view(img.view, None) };
                unsafe { self.device.destroy_image(img.image, None) };
                let _ = allocator.free(img.alloc);
            }
        }
    }

    fn destroy_swapchain_resources(&mut self) {
        self.destroy_gbuffer();
        unsafe {
            for view in self.swap_views.drain(..) {
                self.device.destroy_image_view(view, None);
            }
        }
        self.swap_images.clear();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
        self.destroy_swapchain_resources();
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
        let mut allocator = self.allocator.take().unwrap();
        for frame in self.frames.drain(..) {
            unsafe {
                self.device.destroy_semaphore(frame.image_available, None);
                self.device.destroy_semaphore(frame.render_finished, None);
                self.device.destroy_fence(frame.in_flight, None);
                self.device.destroy_buffer(frame.ubo.buffer, None);
            }
            if let Some(alloc) = frame.ubo.alloc {
                let _ = allocator.free(alloc);
            }
        }
        unsafe {
            for mesh in &mut self.meshes {
                self.device.destroy_buffer(mesh.vb.buffer, None);
                self.device.destroy_buffer(mesh.ib.buffer, None);
            }
        }
        for mesh in self.meshes.drain(..) {
            if let Some(alloc) = mesh.vb.alloc {
                let _ = allocator.free(alloc);
            }
            if let Some(alloc) = mesh.ib.alloc {
                let _ = allocator.free(alloc);
            }
        }
        drop(allocator);
        unsafe {
            self.device.destroy_pipeline(self.gbuffer_pipe, None);
            self.device.destroy_pipeline(self.lighting_pipe, None);
            self.device.destroy_pipeline_layout(self.gbuffer_pipe_layout, None);
            self.device.destroy_pipeline_layout(self.lighting_pipe_layout, None);
            self.device.destroy_descriptor_set_layout(self.ubo_layout, None);
            self.device.destroy_descriptor_set_layout(self.sampled_layout, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_render_pass(self.gbuffer_pass, None);
            self.device.destroy_render_pass(self.lighting_pass, None);
            self.device.destroy_command_pool(self.cmd_pool, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

fn pick_gpu(
    instance: &Instance,
    surface_loader: &khr_surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32), RenderError> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    let mut best = None;
    for phys in devices {
        let props = unsafe { instance.get_physical_device_properties(phys) };
        let queues = unsafe { instance.get_physical_device_queue_family_properties(phys) };
        for (i, q) in queues.iter().enumerate() {
            let present = unsafe {
                surface_loader.get_physical_device_surface_support(phys, i as u32, surface)?
            };
            if q.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present {
                let score = match props.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 2,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    _ => 0,
                };
                if best.as_ref().map(|(_, _, s)| *s).unwrap_or(-1) < score {
                    best = Some((phys, i as u32, score));
                }
            }
        }
    }
    best.map(|(p, q, _)| (p, q)).ok_or(RenderError::NoGpu)
}

fn create_swapchain(
    instance: &Instance,
    device: &Device,
    surface_loader: &khr_surface::Instance,
    swapchain_loader: &khr_swapchain::Device,
    physical: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    queue_family: u32,
    old: vk::SwapchainKHR,
    requested: vk::Extent2D,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>, vk::Format, vk::Extent2D), RenderError> {
    let caps = unsafe { surface_loader.get_physical_device_surface_capabilities(physical, surface)? };
    let formats = unsafe { surface_loader.get_physical_device_surface_formats(physical, surface)? };
    let present_modes =
        unsafe { surface_loader.get_physical_device_surface_present_modes(physical, surface)? };
    let surface_format = formats
        .iter()
        .copied()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .or_else(|| {
            formats.iter().copied().find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
        })
        .unwrap_or(formats[0]);
    let present_mode = if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    };
    let extent = if caps.current_extent.width != u32::MAX {
        caps.current_extent
    } else {
        vk::Extent2D {
            width: requested.width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: requested
                .height
                .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    };
    let mut image_count = caps.min_image_count + 1;
    if caps.max_image_count > 0 {
        image_count = image_count.min(caps.max_image_count);
    }
    let info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(caps.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old);
    let _ = (instance, queue_family);
    let swapchain = unsafe { swapchain_loader.create_swapchain(&info, None)? };
    let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
    let mut views = Vec::new();
    for image in &images {
        views.push(unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?
        });
    }
    Ok((swapchain, images, views, surface_format.format, extent))
}

fn create_gbuffer_pass(device: &Device) -> Result<vk::RenderPass, RenderError> {
    let color = vk::AttachmentDescription::default()
        .format(vk::Format::R8G8B8A8_UNORM)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let depth = vk::AttachmentDescription::default()
        .format(vk::Format::D32_SFLOAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let attachments = [color, color, depth];
    let color_refs = [
        vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        },
        vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        },
    ];
    let depth_ref = vk::AttachmentReference {
        attachment: 2,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_refs)
        .depth_stencil_attachment(&depth_ref);
    let dep = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );
    let dep2 = vk::SubpassDependency::default()
        .src_subpass(0)
        .dst_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
        )
        .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
        .src_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
        .dst_access_mask(vk::AccessFlags::SHADER_READ);
    let deps = [dep, dep2];
    Ok(unsafe {
        device.create_render_pass(
            &vk::RenderPassCreateInfo::default()
                .attachments(&attachments)
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(&deps),
            None,
        )?
    })
}

fn create_lighting_pass(device: &Device, format: vk::Format) -> Result<vk::RenderPass, RenderError> {
    let color = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
    let color_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_ref));
    let dep = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
    Ok(unsafe {
        device.create_render_pass(
            &vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color))
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(std::slice::from_ref(&dep)),
            None,
        )?
    })
}

fn create_sampler(device: &Device) -> Result<vk::Sampler, RenderError> {
    Ok(unsafe {
        device.create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE),
            None,
        )?
    })
}

fn create_set_layouts(
    device: &Device,
) -> Result<(vk::DescriptorSetLayout, vk::DescriptorSetLayout), RenderError> {
    let ubo = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT);
    let ubo_layout = unsafe {
        device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&ubo)),
            None,
        )?
    };
    let sampled = [
        vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default()
            .binding(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];
    let sampled_layout = unsafe {
        device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&sampled),
            None,
        )?
    };
    Ok((ubo_layout, sampled_layout))
}

fn load_shader(device: &Device, bytes: &[u8]) -> Result<vk::ShaderModule, RenderError> {
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    Ok(unsafe {
        device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)?
    })
}

fn shader_stage<'a>(
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
    name: &'a CStr,
) -> vk::PipelineShaderStageCreateInfo<'a> {
    vk::PipelineShaderStageCreateInfo::default()
        .stage(stage)
        .module(module)
        .name(name)
}

fn create_gbuffer_pipeline(
    device: &Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    vert: vk::ShaderModule,
    frag: vk::ShaderModule,
) -> Result<vk::Pipeline, RenderError> {
    let name = CStr::from_bytes_with_nul(b"main\0").unwrap();
    let stages = [
        shader_stage(vert, vk::ShaderStageFlags::VERTEX, name),
        shader_stage(frag, vk::ShaderStageFlags::FRAGMENT, name),
    ];
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(size_of::<Vertex>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX);
    let attrs = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 0,
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 12,
        },
    ];
    let vertex = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(std::slice::from_ref(&binding))
        .vertex_attribute_descriptions(&attrs);
    let input_asm = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let raster = vk::PipelineRasterizationStateCreateInfo::default()
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0);
    let msaa = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let depth = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS);
    let blend_att = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    let blends = [blend_att, blend_att];
    let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blends);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex)
        .input_assembly_state(&input_asm)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&msaa)
        .depth_stencil_state(&depth)
        .color_blend_state(&blend)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipes = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
    }
    .map_err(|(_, e)| e)?;
    Ok(pipes[0])
}

fn create_lighting_pipeline(
    device: &Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    vert: vk::ShaderModule,
    frag: vk::ShaderModule,
) -> Result<vk::Pipeline, RenderError> {
    let name = CStr::from_bytes_with_nul(b"main\0").unwrap();
    let stages = [
        shader_stage(vert, vk::ShaderStageFlags::VERTEX, name),
        shader_stage(frag, vk::ShaderStageFlags::FRAGMENT, name),
    ];
    let vertex = vk::PipelineVertexInputStateCreateInfo::default();
    let input_asm = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let raster = vk::PipelineRasterizationStateCreateInfo::default()
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0);
    let msaa = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let blend_att = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    let blend = vk::PipelineColorBlendStateCreateInfo::default()
        .attachments(std::slice::from_ref(&blend_att));
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex)
        .input_assembly_state(&input_asm)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&msaa)
        .color_blend_state(&blend)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipes = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
    }
    .map_err(|(_, e)| e)?;
    Ok(pipes[0])
}

fn create_buffer(
    device: &Device,
    allocator: &mut Allocator,
    size: u64,
    usage: vk::BufferUsageFlags,
    location: MemoryLocation,
    name: &str,
) -> Result<GpuBuffer, RenderError> {
    let buffer = unsafe {
        device.create_buffer(
            &vk::BufferCreateInfo::default()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )?
    };
    let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let alloc = allocator.allocate(&AllocationCreateDesc {
        name,
        requirements,
        location,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    unsafe { device.bind_buffer_memory(buffer, alloc.memory(), alloc.offset())? };
    Ok(GpuBuffer {
        buffer,
        alloc: Some(alloc),
    })
}

fn upload_buffer(
    device: &Device,
    queue: vk::Queue,
    cmd_pool: vk::CommandPool,
    allocator: &mut Allocator,
    data: &[u8],
    usage: vk::BufferUsageFlags,
    name: &str,
) -> Result<GpuBuffer, RenderError> {
    let mut staging = create_buffer(
        device,
        allocator,
        data.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
        "staging",
    )?;
    unsafe {
        staging
            .alloc
            .as_ref()
            .and_then(|a| a.mapped_ptr())
            .ok_or_else(|| RenderError::Alloc("staging".into()))?
            .as_ptr()
            .cast::<u8>()
            .copy_from_nonoverlapping(data.as_ptr(), data.len());
    }
    let dst = create_buffer(
        device,
        allocator,
        data.len() as u64,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        MemoryLocation::GpuOnly,
        name,
    )?;
    let cmd = unsafe {
        device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(cmd_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?[0]
    };
    unsafe {
        device.begin_command_buffer(
            cmd,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;
        device.cmd_copy_buffer(
            cmd,
            staging.buffer,
            dst.buffer,
            &[vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: data.len() as u64,
            }],
        );
        device.end_command_buffer(cmd)?;
        device.queue_submit(
            queue,
            &[vk::SubmitInfo::default().command_buffers(&[cmd])],
            vk::Fence::null(),
        )?;
        device.queue_wait_idle(queue)?;
        device.free_command_buffers(cmd_pool, &[cmd]);
        device.destroy_buffer(staging.buffer, None);
    }
    if let Some(alloc) = staging.alloc.take() {
        allocator.free(alloc)?;
    }
    Ok(dst)
}

fn create_color_target(
    device: &Device,
    allocator: &mut Allocator,
    extent: vk::Extent2D,
    format: vk::Format,
    name: &str,
) -> Result<GpuImage, RenderError> {
    create_image(
        device,
        allocator,
        extent,
        format,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::ImageAspectFlags::COLOR,
        name,
    )
}

fn create_depth_target(
    device: &Device,
    allocator: &mut Allocator,
    extent: vk::Extent2D,
) -> Result<GpuImage, RenderError> {
    create_image(
        device,
        allocator,
        extent,
        vk::Format::D32_SFLOAT,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::ImageAspectFlags::DEPTH,
        "depth",
    )
}

fn create_image(
    device: &Device,
    allocator: &mut Allocator,
    extent: vk::Extent2D,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
    name: &str,
) -> Result<GpuImage, RenderError> {
    let image = unsafe {
        device.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width: extent.width,
                    height: extent.height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?
    };
    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let alloc = allocator.allocate(&AllocationCreateDesc {
        name,
        requirements,
        location: MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    unsafe { device.bind_image_memory(image, alloc.memory(), alloc.offset())? };
    let view = unsafe {
        device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }),
            None,
        )?
    };
    Ok(GpuImage { image, view, alloc })
}
