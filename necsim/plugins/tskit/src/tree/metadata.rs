use std::{array::TryFromSliceError, convert::TryInto, io};

use tskit::metadata::{IndividualMetadata, MetadataError, MetadataRoundtrip, NodeMetadata};

use necsim_core::lineage::GlobalLineageReference;

#[expect(clippy::module_name_repetitions)]
#[repr(transparent)]
pub struct GlobalLineageMetadata(GlobalLineageReference);

impl MetadataRoundtrip for GlobalLineageMetadata {
    fn encode(&self) -> Result<Vec<u8>, MetadataError> {
        // Store the internal u64
        Ok(unsafe { self.0.clone().into_inner() }
            .to_le_bytes()
            .to_vec())
    }

    fn decode(metadata: &[u8]) -> Result<Self, MetadataError>
    where
        Self: Sized,
    {
        // Ensure that `metadata` contains exactly eight bytes
        let value_bytes: [u8; 8] = metadata.try_into().map_err(|err: TryFromSliceError| {
            MetadataError::RoundtripError {
                value: Box::new(io::Error::new(io::ErrorKind::InvalidData, err.to_string())),
            }
        })?;

        // Convert the bytes into an u64 and a GlobalLineageReference
        Ok(Self(unsafe {
            GlobalLineageReference::from_inner(u64::from_le_bytes(value_bytes))
        }))
    }
}

impl IndividualMetadata for GlobalLineageMetadata {}
impl NodeMetadata for GlobalLineageMetadata {}

impl GlobalLineageMetadata {
    pub fn new(reference: &GlobalLineageReference) -> &Self {
        unsafe { &*std::ptr::from_ref(reference).cast() }
    }
}
