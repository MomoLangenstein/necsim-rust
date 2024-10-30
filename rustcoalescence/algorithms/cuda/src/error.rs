use rust_cuda::deps::rustacuda::error::CudaError as RustaCudaError;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
#[serde(into = "CudaErrorRaw", from = "CudaErrorRaw")]
#[error(transparent)]
#[expect(clippy::module_name_repetitions)]
pub struct CudaError(#[from] RustaCudaError);

#[derive(Serialize, Deserialize)]
#[serde(rename = "CudaError")]
struct CudaErrorRaw {
    code: u32,
}

impl From<CudaError> for CudaErrorRaw {
    fn from(error: CudaError) -> Self {
        Self {
            code: error.0 as u32,
        }
    }
}

impl From<CudaErrorRaw> for CudaError {
    fn from(value: CudaErrorRaw) -> Self {
        type E = RustaCudaError;

        #[expect(clippy::wildcard_in_or_patterns)]
        let code = match value.code {
            const { E::InvalidValue as u32 } => E::InvalidValue,
            const { E::OutOfMemory as u32 } => E::OutOfMemory,
            const { E::NotInitialized as u32 } => E::NotInitialized,
            const { E::Deinitialized as u32 } => E::Deinitialized,
            const { E::ProfilerDisabled as u32 } => E::ProfilerDisabled,
            const { E::ProfilerNotInitialized as u32 } => E::ProfilerNotInitialized,
            const { E::ProfilerAlreadyStarted as u32 } => E::ProfilerAlreadyStarted,
            const { E::ProfilerAlreadyStopped as u32 } => E::ProfilerAlreadyStopped,
            const { E::NoDevice as u32 } => E::NoDevice,
            const { E::InvalidDevice as u32 } => E::InvalidDevice,
            const { E::InvalidImage as u32 } => E::InvalidImage,
            const { E::InvalidContext as u32 } => E::InvalidContext,
            const { E::ContextAlreadyCurrent as u32 } => E::ContextAlreadyCurrent,
            const { E::MapFailed as u32 } => E::MapFailed,
            const { E::UnmapFailed as u32 } => E::UnmapFailed,
            const { E::ArrayIsMapped as u32 } => E::ArrayIsMapped,
            const { E::AlreadyMapped as u32 } => E::AlreadyMapped,
            const { E::NoBinaryForGpu as u32 } => E::NoBinaryForGpu,
            const { E::AlreadyAcquired as u32 } => E::AlreadyAcquired,
            const { E::NotMapped as u32 } => E::NotMapped,
            const { E::NotMappedAsArray as u32 } => E::NotMappedAsArray,
            const { E::NotMappedAsPointer as u32 } => E::NotMappedAsPointer,
            const { E::EccUncorrectable as u32 } => E::EccUncorrectable,
            const { E::UnsupportedLimit as u32 } => E::UnsupportedLimit,
            const { E::ContextAlreadyInUse as u32 } => E::ContextAlreadyInUse,
            const { E::PeerAccessUnsupported as u32 } => E::PeerAccessUnsupported,
            const { E::InvalidPtx as u32 } => E::InvalidPtx,
            const { E::InvalidGraphicsContext as u32 } => E::InvalidGraphicsContext,
            const { E::NvlinkUncorrectable as u32 } => E::NvlinkUncorrectable,
            const { E::InvalidSouce as u32 } => E::InvalidSouce,
            const { E::FileNotFound as u32 } => E::FileNotFound,
            const { E::SharedObjectSymbolNotFound as u32 } => E::SharedObjectSymbolNotFound,
            const { E::SharedObjectInitFailed as u32 } => E::SharedObjectInitFailed,
            const { E::OperatingSystemError as u32 } => E::OperatingSystemError,
            const { E::InvalidHandle as u32 } => E::InvalidHandle,
            const { E::NotFound as u32 } => E::NotFound,
            const { E::NotReady as u32 } => E::NotReady,
            const { E::IllegalAddress as u32 } => E::IllegalAddress,
            const { E::LaunchOutOfResources as u32 } => E::LaunchOutOfResources,
            const { E::LaunchTimeout as u32 } => E::LaunchTimeout,
            const { E::LaunchIncompatibleTexturing as u32 } => E::LaunchIncompatibleTexturing,
            const { E::PeerAccessAlreadyEnabled as u32 } => E::PeerAccessAlreadyEnabled,
            const { E::PeerAccessNotEnabled as u32 } => E::PeerAccessNotEnabled,
            const { E::PrimaryContextActive as u32 } => E::PrimaryContextActive,
            const { E::ContextIsDestroyed as u32 } => E::ContextIsDestroyed,
            const { E::AssertError as u32 } => E::AssertError,
            const { E::TooManyPeers as u32 } => E::TooManyPeers,
            const { E::HostMemoryAlreadyRegistered as u32 } => E::HostMemoryAlreadyRegistered,
            const { E::HostMemoryNotRegistered as u32 } => E::HostMemoryNotRegistered,
            const { E::HardwareStackError as u32 } => E::HardwareStackError,
            const { E::IllegalInstruction as u32 } => E::IllegalInstruction,
            const { E::MisalignedAddress as u32 } => E::MisalignedAddress,
            const { E::InvalidAddressSpace as u32 } => E::InvalidAddressSpace,
            const { E::InvalidProgramCounter as u32 } => E::InvalidProgramCounter,
            const { E::LaunchFailed as u32 } => E::LaunchFailed,
            const { E::NotPermitted as u32 } => E::NotPermitted,
            const { E::NotSupported as u32 } => E::NotSupported,
            const { E::InvalidMemoryAllocation as u32 } => E::InvalidMemoryAllocation,
            const { E::UnknownError as u32 } | _ => E::UnknownError,
        };

        Self(code)
    }
}
