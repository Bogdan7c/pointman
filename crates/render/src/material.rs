//! Один descriptor set на материал: albedo + normal + spec.
//! Vulkan 1.1 / Android — без bindless, один rebind на смену материала.

use crate::error::RenderError;
use ash::{vk, Device};

pub const MAX_MATERIALS: u32 = 1024;

fn sampler_binding(binding: u32) -> vk::DescriptorSetLayoutBinding<'static> {
    vk::DescriptorSetLayoutBinding::default()
        .binding(binding)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
}

pub fn create_material_layout(device: &Device) -> Result<vk::DescriptorSetLayout, RenderError> {
    let bindings = [sampler_binding(0), sampler_binding(1), sampler_binding(2)];
    Ok(unsafe {
        device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
            None,
        )?
    })
}

pub fn write_material_set(
    device: &Device,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
    sampler: vk::Sampler,
    albedo: vk::ImageView,
    normal: vk::ImageView,
    spec: vk::ImageView,
) -> Result<vk::DescriptorSet, RenderError> {
    let set = unsafe {
        device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(std::slice::from_ref(&layout)),
        )?[0]
    };
    let infos = [
        image_info(sampler, albedo),
        image_info(sampler, normal),
        image_info(sampler, spec),
    ];
    let writes = [
        write_sampled(set, 0, &infos[0]),
        write_sampled(set, 1, &infos[1]),
        write_sampled(set, 2, &infos[2]),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]) };
    Ok(set)
}

fn image_info(sampler: vk::Sampler, view: vk::ImageView) -> vk::DescriptorImageInfo {
    vk::DescriptorImageInfo::default()
        .sampler(sampler)
        .image_view(view)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
}

fn write_sampled(
    set: vk::DescriptorSet,
    binding: u32,
    info: &vk::DescriptorImageInfo,
) -> vk::WriteDescriptorSet<'_> {
    vk::WriteDescriptorSet::default()
        .dst_set(set)
        .dst_binding(binding)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(std::slice::from_ref(info))
}
