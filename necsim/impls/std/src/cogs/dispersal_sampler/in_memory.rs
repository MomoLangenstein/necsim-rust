use necsim_impls_no_std::cogs::dispersal_sampler::in_memory::InMemoryDispersalSamplerError as InMemoryDispersalSamplerErrorNoStd;

use thiserror::Error;

#[expect(clippy::module_name_repetitions)]
#[derive(Error, Debug)]
#[error("{0}")]
pub struct InMemoryDispersalSamplerError(pub InMemoryDispersalSamplerErrorNoStd);

impl From<InMemoryDispersalSamplerErrorNoStd> for InMemoryDispersalSamplerError {
    fn from(err: InMemoryDispersalSamplerErrorNoStd) -> Self {
        Self(err)
    }
}
