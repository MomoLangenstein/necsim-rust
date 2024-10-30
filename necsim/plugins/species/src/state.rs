use bincode::Options;
use serde::{Deserialize, Serialize};

use necsim_core::{
    event::{DispersalEvent, SpeciationEvent},
    lineage::GlobalLineageReference,
};
use necsim_core_bond::NonNegativeF64;

#[expect(clippy::module_name_repetitions, clippy::struct_field_names)]
#[derive(Serialize, Deserialize)]
pub struct LastEventState {
    pub last_parent_prior_time: Option<(GlobalLineageReference, NonNegativeF64)>,
    pub last_speciation_event: Option<SpeciationEvent>,
    pub last_dispersal_event: Option<DispersalEvent>,
}

impl LastEventState {
    pub fn into_string(self) -> Result<String, ()> {
        let bytes = bincode::options().serialize(&self).map_err(|_| ())?;

        Ok(base32::encode(base32::Alphabet::Crockford, &bytes))
    }

    pub fn from_string(string: &str) -> Result<LastEventState, ()> {
        let bytes = base32::decode(base32::Alphabet::Crockford, string).ok_or(())?;

        bincode::options().deserialize(&bytes).map_err(|_| ())
    }
}
