use rust_cuda::deps::rustacuda::{
    context::{Context, CurrentContext, ResourceLimit},
    prelude::*,
};

use rust_cuda::host::CudaDropWrapper;

use crate::{error::CudaError, info};

#[expect(clippy::module_name_repetitions)]
pub fn with_initialised_cuda<O, E: Into<CudaError>, F: FnOnce() -> Result<O, E>>(
    device: u32,
    inner: F,
) -> Result<O, CudaError> {
    // Initialize the CUDA API
    rust_cuda::deps::rustacuda::init(CudaFlags::empty())?;

    // Get the first device
    let device = Device::get_device(device)?;

    // Create a context associated to this device
    let context = CudaDropWrapper::from(Context::create_and_push(
        ContextFlags::MAP_HOST | ContextFlags::SCHED_AUTO,
        device,
    )?);

    CurrentContext::set_resource_limit(ResourceLimit::StackSize, 4096)?;

    info::print_context_resource_limits();

    let result = inner();

    // Explicit drop of the current CUDA context to explicitly end its scope
    std::mem::drop(context);

    result.map_err(E::into)
}
