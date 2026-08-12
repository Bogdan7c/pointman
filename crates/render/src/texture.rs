use crate::error::RenderError;
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;

pub const MAX_TEXTURES: u32 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TextureId(pub u32);

impl TextureId {
    pub const WHITE: Self = Self(0);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureFormat {
    Rgba8,
    Bc1,
    Bc2,
    Bc3,
}

pub struct TextureUpload<'a> {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: TextureFormat,
    pub bytes: &'a [u8],
}

pub(crate) struct GpuTexture {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub alloc: Allocation,
    pub set: vk::DescriptorSet,
}

pub(crate) fn create_material_layout(device: &Device) -> Result<vk::DescriptorSetLayout, RenderError> {
    let binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);
    Ok(unsafe {
        device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding)),
            None,
        )?
    })
}

pub(crate) fn upload_texture(
    device: &Device,
    queue: vk::Queue,
    cmd_pool: vk::CommandPool,
    allocator: &mut Allocator,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    sampler: vk::Sampler,
    data: TextureUpload<'_>,
) -> Result<GpuTexture, RenderError> {
    let format = vk_format(data.format);
    let mip_count = data.mip_count.max(1);
    let image = unsafe {
        device.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width: data.width,
                    height: data.height,
                    depth: 1,
                })
                .mip_levels(mip_count)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?
    };
    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let alloc = allocator.allocate(&AllocationCreateDesc {
        name: "texture",
        requirements,
        location: MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    unsafe { device.bind_image_memory(image, alloc.memory(), alloc.offset())? };

    let mut staging = create_staging(
        device,
        allocator,
        data.bytes.len() as u64,
    )?;
    unsafe {
        staging
            .alloc
            .as_ref()
            .and_then(|a| a.mapped_ptr())
            .ok_or_else(|| RenderError::Alloc("tex staging".into()))?
            .as_ptr()
            .cast::<u8>()
            .copy_from_nonoverlapping(data.bytes.as_ptr(), data.bytes.len());
    }

    let mut copies = Vec::new();
    let mut offset = 0u64;
    let mut w = data.width;
    let mut h = data.height;
    for mip in 0..mip_count {
        let size = mip_bytes(w, h, data.format) as u64;
        copies.push(
            vk::BufferImageCopy::default()
                .buffer_offset(offset)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: mip,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D {
                    width: w.max(1),
                    height: h.max(1),
                    depth: 1,
                }),
        );
        offset += size;
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }

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
        let barrier_to_dst = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_count,
                base_array_layer: 0,
                layer_count: 1,
            });
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_dst),
        );
        device.cmd_copy_buffer_to_image(
            cmd,
            staging.buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &copies,
        );
        let barrier_to_sample = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_count,
                base_array_layer: 0,
                layer_count: 1,
            });
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_sample),
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

    let view = unsafe {
        device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: mip_count,
                    base_array_layer: 0,
                    layer_count: 1,
                }),
            None,
        )?
    };
    let set = unsafe {
        device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(std::slice::from_ref(&layout)),
        )?[0]
    };
    let image_info = vk::DescriptorImageInfo::default()
        .sampler(sampler)
        .image_view(view)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let write = vk::WriteDescriptorSet::default()
        .dst_set(set)
        .dst_binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(std::slice::from_ref(&image_info));
    unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
    Ok(GpuTexture {
        image,
        view,
        alloc,
        set,
    })
}

pub(crate) fn white_upload() -> TextureUpload<'static> {
    TextureUpload {
        width: 1,
        height: 1,
        mip_count: 1,
        format: TextureFormat::Rgba8,
        bytes: &[255, 255, 255, 255],
    }
}

fn vk_format(format: TextureFormat) -> vk::Format {
    match format {
        TextureFormat::Rgba8 => vk::Format::R8G8B8A8_UNORM,
        TextureFormat::Bc1 => vk::Format::BC1_RGBA_UNORM_BLOCK,
        TextureFormat::Bc2 => vk::Format::BC2_UNORM_BLOCK,
        TextureFormat::Bc3 => vk::Format::BC3_UNORM_BLOCK,
    }
}

fn mip_bytes(width: u32, height: u32, format: TextureFormat) -> usize {
    match format {
        TextureFormat::Rgba8 => width as usize * height as usize * 4,
        TextureFormat::Bc1 => {
            let bx = (width.max(1) + 3) / 4;
            let by = (height.max(1) + 3) / 4;
            bx as usize * by as usize * 8
        }
        TextureFormat::Bc2 | TextureFormat::Bc3 => {
            let bx = (width.max(1) + 3) / 4;
            let by = (height.max(1) + 3) / 4;
            bx as usize * by as usize * 16
        }
    }
}

struct Staging {
    buffer: vk::Buffer,
    alloc: Option<Allocation>,
}

fn create_staging(
    device: &Device,
    allocator: &mut Allocator,
    size: u64,
) -> Result<Staging, RenderError> {
    let buffer = unsafe {
        device.create_buffer(
            &vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )?
    };
    let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let alloc = allocator.allocate(&AllocationCreateDesc {
        name: "tex-staging",
        requirements,
        location: MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    unsafe { device.bind_buffer_memory(buffer, alloc.memory(), alloc.offset())? };
    Ok(Staging {
        buffer,
        alloc: Some(alloc),
    })
}
