use serde::{Serialize, Serializer};

use necsim_partitioning_core::partition::PartitionSize;

#[cfg(any(
    feature = "gillespie-algorithms",
    feature = "independent-algorithm",
    feature = "cuda-algorithm"
))]
use rustcoalescence_algorithms::AlgorithmParamters;

#[derive(Debug, DeserializeState)]
#[serde(deserialize_state = "PartitionSize")]
pub enum Algorithm {
    #[cfg(feature = "gillespie-algorithms")]
    #[serde(alias = "Classical")]
    Gillespie(
        #[serde(deserialize_state)]
        <rustcoalescence_algorithms_gillespie::gillespie::GillespieAlgorithm as rustcoalescence_algorithms::AlgorithmParamters>::Arguments,
    ),
    #[cfg(feature = "gillespie-algorithms")]
    #[serde(alias = "SkippingGillespie")]
    EventSkipping(
        #[serde(deserialize_state)]
        <rustcoalescence_algorithms_gillespie::event_skipping::EventSkippingAlgorithm as AlgorithmParamters>::Arguments,
    ),
    #[cfg(feature = "cuda-algorithm")]
    #[serde(alias = "CUDA")]
    Cuda(#[serde(deserialize_state)] <rustcoalescence_algorithms_cuda::CudaAlgorithm as AlgorithmParamters>::Arguments),
    #[cfg(feature = "independent-algorithm")]
    Independent(
        #[serde(deserialize_state)] <rustcoalescence_algorithms_independent::IndependentAlgorithm as AlgorithmParamters>::Arguments,
    ),
}

impl Serialize for Algorithm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unreachable_patterns)] // FIXME: use expect
        match self {
            #[cfg(feature = "gillespie-algorithms")]
            Self::Gillespie(args) => {
                serializer.serialize_newtype_variant(stringify!(Algorithm), 0, "Gillespie", args)
            },
            #[cfg(feature = "gillespie-algorithms")]
            Self::EventSkipping(args) => serializer.serialize_newtype_variant(
                stringify!(Algorithm),
                1,
                "EventSkipping",
                args,
            ),
            #[cfg(feature = "cuda-algorithm")]
            Self::Cuda(args) => {
                serializer.serialize_newtype_variant(stringify!(Algorithm), 2, "CUDA", args)
            },
            #[cfg(feature = "independent-algorithm")]
            Self::Independent(args) => {
                serializer.serialize_newtype_variant(stringify!(Algorithm), 3, "Independent", args)
            },
            _ => {
                std::mem::drop(serializer);

                Err(serde::ser::Error::custom(
                    "rustcoalescence must be compiled to support at least one algorithm",
                ))
            },
        }
    }
}
