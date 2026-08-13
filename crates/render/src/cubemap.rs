//! GPU cubemap: шесть граней, `samplerCube`. Не смешивать с плоскими TextureId стен.

use crate::error::RenderError;
use crate::texture::{self, GpuTexture, TextureFormat};
use ash::{vk, Device};
use gpu_allocator::vulkan::Allocator;

/// Слот неба на GPU. Сейчас один cubemap на кадр.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CubemapId(pub u32);

impl CubemapId {
    pub const SKY: Self = Self(0);
}

pub struct CubemapUpload<'a> {
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub format: TextureFormat,
    /// Все грани подряд, как в D3D9 DDS: +X −X +Y −Y +Z −Z, у каждой свои mip.
    pub bytes: &'a [u8],
}

pub(crate) fn dummy_upload() -> CubemapUpload<'static> {
    // 1×1, пока уровень не принёс настоящее небо. Lighting не семплит, если sky выключен.
    CubemapUpload {
        width: 1,
        height: 1,
        mip_count: 1,
        format: TextureFormat::Rgba8,
        bytes: &[
            40, 40, 45, 255, 40, 40, 45, 255, 40, 40, 45, 255, 40, 40, 45, 255, 40, 40, 45, 255,
            40, 40, 45, 255,
        ],
    }
}

pub(crate) fn create_cube_sampler(device: &Device) -> Result<vk::Sampler, RenderError> {
    Ok(unsafe {
        device.create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .max_lod(16.0),
            None,
        )?
    })
}

pub(crate) fn bind_cubemap(
    device: &Device,
    set: vk::DescriptorSet,
    sampler: vk::Sampler,
    view: vk::ImageView,
) {
    let info = vk::DescriptorImageInfo::default()
        .sampler(sampler)
        .image_view(view)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let write = vk::WriteDescriptorSet::default()
        .dst_set(set)
        .dst_binding(4)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(std::slice::from_ref(&info));
    unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
}

pub(crate) fn destroy_cubemap(
    device: &Device,
    allocator: &mut Allocator,
    gpu: GpuTexture,
) -> Result<(), RenderError> {
    unsafe {
        device.destroy_image_view(gpu.view, None);
        device.destroy_image(gpu.image, None);
    }
    allocator.free(gpu.alloc)?;
    Ok(())
}

pub(crate) fn upload_cubemap(
    device: &Device,
    queue: vk::Queue,
    cmd_pool: vk::CommandPool,
    allocator: &mut Allocator,
    data: CubemapUpload<'_>,
) -> Result<GpuTexture, RenderError> {
    let format = texture::vk_format(data.format);
    let mip_count = data.mip_count.max(1);
    let image = unsafe {
        device.create_image(
            &vk::ImageCreateInfo::default()
                .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE)
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width: data.width,
                    height: data.height,
                    depth: 1,
                })
                .mip_levels(mip_count)
                .array_layers(6)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?
    };
    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let alloc = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "cubemap",
        requirements,
        location: gpu_allocator::MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
    })?;
    unsafe { device.bind_image_memory(image, alloc.memory(), alloc.offset())? };

    let mut staging = texture::create_staging(device, allocator, data.bytes.len() as u64)?;
    unsafe {
        staging
            .alloc
            .as_ref()
            .and_then(|a| a.mapped_ptr())
            .ok_or_else(|| RenderError::Alloc("cube staging".into()))?
            .as_ptr()
            .cast::<u8>()
            .copy_from_nonoverlapping(data.bytes.as_ptr(), data.bytes.len());
    }

    let mut copies = Vec::new();
    let mut offset = 0u64;
    for face in 0..6u32 {
        let mut w = data.width;
        let mut h = data.height;
        for mip in 0..mip_count {
            let size = texture::mip_bytes(w, h, data.format) as u64;
            copies.push(
                vk::BufferImageCopy::default()
                    .buffer_offset(offset)
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: mip,
                        base_array_layer: face,
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
    }

    let cmd = unsafe {
        device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(cmd_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?[0]
    };
    let cube_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: mip_count,
        base_array_layer: 0,
        layer_count: 6,
    };
    unsafe {
        device.begin_command_buffer(
            cmd,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;
        let to_dst = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image)
            .subresource_range(cube_range);
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&to_dst),
        );
        device.cmd_copy_buffer_to_image(
            cmd,
            staging.buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &copies,
        );
        let to_sample = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image)
            .subresource_range(cube_range);
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&to_sample),
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
                .view_type(vk::ImageViewType::CUBE)
                .format(format)
                .subresource_range(cube_range),
            None,
        )?
    };
    Ok(GpuTexture {
        image,
        view,
        alloc,
    })
}
