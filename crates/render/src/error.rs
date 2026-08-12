use ash::vk;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("vulkan load: {0}")]
    Load(#[from] ash::LoadingError),
    #[error("vulkan: {0}")]
    Vulkan(vk::Result),
    #[error("no suitable GPU")]
    NoGpu,
    #[error("allocator: {0}")]
    Alloc(String),
    #[error("window handle")]
    Window,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<vk::Result> for RenderError {
    fn from(value: vk::Result) -> Self {
        Self::Vulkan(value)
    }
}

impl From<gpu_allocator::AllocationError> for RenderError {
    fn from(value: gpu_allocator::AllocationError) -> Self {
        Self::Alloc(value.to_string())
    }
}
