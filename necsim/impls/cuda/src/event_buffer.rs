use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use const_type_layout::TypeGraphLayout;
#[cfg(not(target_os = "cuda"))]
use rust_cuda::deps::rustacuda::{
    error::CudaResult,
    function::{BlockSize, GridSize},
};

use rust_cuda::{
    lend::RustToCudaProxy,
    safety::{PortableBitSemantics, SafeMutableAliasing, StackOnly},
    utils::{
        aliasing::SplitSliceOverCudaThreadsDynamicStride,
        exchange::buffer::{CudaExchangeBuffer, CudaExchangeItem},
    },
};

use necsim_core::{
    event::{PackedEvent, SpeciationEvent, TypedEvent},
    reporter::{
        boolean::{Boolean, False, True},
        Reporter,
    },
};

#[cfg(target_os = "cuda")]
use necsim_core::impl_report;

use super::utils::MaybeSome;

#[allow(clippy::module_name_repetitions, clippy::type_complexity)] // FIXME: use expect
#[derive(rust_cuda::lend::LendRustToCuda)]
#[cuda(free = "ReportSpeciation", free = "ReportDispersal")]
pub struct EventBuffer<ReportSpeciation: Boolean, ReportDispersal: Boolean> {
    #[cfg(not(target_os = "cuda"))]
    #[cuda(embed)]
    event_mask: SplitSliceOverCudaThreadsDynamicStride<CudaExchangeBuffer<bool, true, true>>,
    #[cfg(target_os = "cuda")]
    #[cuda(embed = "SplitSliceOverCudaThreadsDynamicStride<CudaExchangeBuffer<bool, true, true>>")]
    event_mask: CudaExchangeSlice<CudaExchangeItem<bool, true, true>>,
    #[cfg(not(target_os = "cuda"))]
    #[cuda(embed)]
    event_buffer: SplitSliceOverCudaThreadsDynamicStride<
        CudaExchangeBuffer<
            MaybeSome<<EventBuffer<ReportSpeciation, ReportDispersal> as EventType>::Event>,
            false,
            true,
        >,
    >,
    #[cfg(target_os = "cuda")]
    #[cuda(embed = "SplitSliceOverCudaThreadsDynamicStride<
    CudaExchangeBuffer<
        MaybeSome<<EventBuffer<ReportSpeciation, ReportDispersal> as EventType>::Event>,
        false,
        true,
    >,
>")]
    event_buffer: CudaExchangeSlice<
        CudaExchangeItem<
            MaybeSome<<EventBuffer<ReportSpeciation, ReportDispersal> as EventType>::Event>,
            false,
            true,
        >,
    >,
}

// Safety:
// - no mutable aliasing occurs since all parts implement SafeMutableAliasing
// - dropping does not trigger (de)alloc since EventBuffer doesn't impl Drop and
//   all parts implement SafeMutableAliasing
// - EventBuffer has no shallow mutable state
unsafe impl<ReportSpeciation: Boolean, ReportDispersal: Boolean> SafeMutableAliasing
    for EventBuffer<ReportSpeciation, ReportDispersal>
where
    SplitSliceOverCudaThreadsDynamicStride<CudaExchangeBuffer<bool, true, true>>:
        SafeMutableAliasing,
    SplitSliceOverCudaThreadsDynamicStride<
        CudaExchangeBuffer<
            MaybeSome<<EventBuffer<ReportSpeciation, ReportDispersal> as EventType>::Event>,
            false,
            true,
        >,
    >: SafeMutableAliasing,
{
}

pub trait EventType {
    type Event: 'static
        + Sync
        + rust_cuda::deps::const_type_layout::TypeGraphLayout
        + rust_cuda::safety::StackOnly
        + rust_cuda::safety::PortableBitSemantics
        + Into<TypedEvent>
        + Into<PackedEvent>
        + Clone;
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean> EventType
    for EventBuffer<ReportSpeciation, ReportDispersal>
{
    default type Event = PackedEvent;
}

impl EventType for EventBuffer<False, False> {
    type Event = PackedEvent;
}

impl EventType for EventBuffer<False, True> {
    type Event = PackedEvent;
}

impl EventType for EventBuffer<True, False> {
    type Event = SpeciationEvent;
}

impl EventType for EventBuffer<True, True> {
    type Event = PackedEvent;
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean> fmt::Debug
    for EventBuffer<ReportSpeciation, ReportDispersal>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("EventBuffer").finish_non_exhaustive()
    }
}

#[cfg(not(target_os = "cuda"))]
impl<ReportSpeciation: Boolean, ReportDispersal: Boolean>
    EventBuffer<ReportSpeciation, ReportDispersal>
{
    /// # Errors
    /// Returns a `rustacuda::errors::CudaError` iff an error occurs inside CUDA
    pub fn new(
        block_size: &BlockSize,
        grid_size: &GridSize,
        max_events: usize,
    ) -> CudaResult<Self> {
        let block_size = (block_size.x * block_size.y * block_size.z) as usize;
        let grid_size = (grid_size.x * grid_size.y * grid_size.z) as usize;

        #[expect(clippy::bool_to_int_with_if)]
        let max_events = if ReportDispersal::VALUE {
            max_events
        } else if ReportSpeciation::VALUE {
            1_usize
        } else {
            0_usize
        };

        let event_capacity = max_events * block_size * grid_size;

        let mut event_buffer = alloc::vec::Vec::with_capacity(event_capacity);
        event_buffer.resize_with(event_capacity, || MaybeSome::None);

        Ok(Self {
            event_mask: SplitSliceOverCudaThreadsDynamicStride::new(
                CudaExchangeBuffer::new(&false, event_capacity)?,
                max_events,
            ),
            event_buffer: SplitSliceOverCudaThreadsDynamicStride::new(
                CudaExchangeBuffer::from_vec(event_buffer)?,
                max_events,
            ),
        })
    }

    pub fn report_events_unordered<P>(&mut self, reporter: &mut P)
    where
        P: Reporter<ReportSpeciation = ReportSpeciation, ReportDispersal = ReportDispersal>,
    {
        for (mask, event) in self.event_mask.iter_mut().zip(self.event_buffer.iter()) {
            if *mask.read() {
                let event: TypedEvent = unsafe { event.read().assume_some_read() }.into();

                match event {
                    TypedEvent::Speciation(ref speciation) => {
                        reporter.report_speciation(speciation.into());
                    },
                    TypedEvent::Dispersal(ref dispersal) => {
                        reporter.report_dispersal(dispersal.into());
                    },
                }
            }

            mask.write(false);
        }
    }
}

#[cfg(target_os = "cuda")]
impl<ReportSpeciation: Boolean, ReportDispersal: Boolean>
    EventBuffer<ReportSpeciation, ReportDispersal>
{
    #[must_use]
    pub fn can_buffer_next_event(&self) -> bool {
        !self.event_buffer.is_empty()
    }

    fn report_event(
        &mut self,
        event: impl Into<<EventBuffer<ReportSpeciation, ReportDispersal> as EventType>::Event>,
    ) {
        if let ([mask, mask_rest @ ..], [buffer, buffer_rest @ ..]) = (
            core::mem::take(&mut *self.event_mask),
            core::mem::take(&mut *self.event_buffer),
        ) {
            mask.write(true);
            buffer.write(MaybeSome::Some(event.into()));

            *self.event_mask = mask_rest;
            *self.event_buffer = buffer_rest;
        }
    }
}

#[cfg(target_os = "cuda")]
impl<ReportSpeciation: Boolean, ReportDispersal: Boolean> Reporter
    for EventBuffer<ReportSpeciation, ReportDispersal>
{
    impl_report!([default] speciation(&mut self, _event: Ignored) {});

    impl_report!([default] dispersal(&mut self, _event: Ignored) {});

    impl_report!([default] progress(&mut self, _progress: Ignored) {});
}

#[cfg(target_os = "cuda")]
impl Reporter for EventBuffer<False, True> {
    impl_report!(
        #[debug_requires(
            self.can_buffer_next_event(),
            "does not report extraneous dispersal events"
        )]
        dispersal(&mut self, event: Used) {
            self.report_event(event.clone());
        }
    );
}

#[cfg(target_os = "cuda")]
impl Reporter for EventBuffer<True, False> {
    impl_report!(
        #[debug_requires(
            self.can_buffer_next_event(),
            "does not report extraneous speciation events"
        )]
        speciation(&mut self, event: Used) {
            self.report_event(event.clone());

            *self.event_mask = &mut [];
            *self.event_buffer = &mut [];
        }
    );
}

#[cfg(target_os = "cuda")]
impl Reporter for EventBuffer<True, True> {
    impl_report!(
        #[debug_requires(
            self.can_buffer_next_event(),
            "does not report extraneous speciation events"
        )]
        speciation(&mut self, event: Used) {
            self.report_event(event.clone());

            *self.event_mask = &mut [];
            *self.event_buffer = &mut [];
        }
    );

    impl_report!(
        #[debug_requires(
            self.can_buffer_next_event(),
            "does not report extraneous dispersal events"
        )]
        dispersal(&mut self, event: Used) {
            self.report_event(event.clone());
        }
    );
}

// TODO: find a prettier workaround
struct CudaExchangeSlice<T: 'static + StackOnly + PortableBitSemantics + TypeGraphLayout>(
    &'static mut [T],
);

impl<T: 'static + StackOnly + PortableBitSemantics + TypeGraphLayout> Deref
    for CudaExchangeSlice<T>
{
    type Target = &'static mut [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: 'static + StackOnly + PortableBitSemantics + TypeGraphLayout> DerefMut
    for CudaExchangeSlice<T>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<
        T: 'static + StackOnly + PortableBitSemantics + TypeGraphLayout,
        const M2D: bool,
        const M2H: bool,
    > RustToCudaProxy<CudaExchangeSlice<CudaExchangeItem<T, M2D, M2H>>>
    for SplitSliceOverCudaThreadsDynamicStride<CudaExchangeBuffer<T, M2D, M2H>>
{
    fn from_ref(_val: &CudaExchangeSlice<CudaExchangeItem<T, M2D, M2H>>) -> &Self {
        unsafe { unreachable_cuda_event_buffer_hack() }
    }

    fn from_mut(_val: &mut CudaExchangeSlice<CudaExchangeItem<T, M2D, M2H>>) -> &mut Self {
        unsafe { unreachable_cuda_event_buffer_hack() }
    }

    fn into(mut self) -> CudaExchangeSlice<CudaExchangeItem<T, M2D, M2H>> {
        let slice: &mut [CudaExchangeItem<T, M2D, M2H>] = &mut self;

        let slice = unsafe { core::slice::from_raw_parts_mut(slice.as_mut_ptr(), slice.len()) };

        CudaExchangeSlice(slice)
    }
}

extern "C" {
    fn unreachable_cuda_event_buffer_hack() -> !;
}
