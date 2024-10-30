use alloc::vec::Vec;
use core::{fmt, marker::PhantomData};
use necsim_core_bond::NonNegativeF64;

use necsim_core::{
    event::{PackedEvent, TypedEvent},
    impl_report,
    reporter::Reporter,
};

use necsim_partitioning_core::LocalPartition;

use super::WaterLevelReporterProxy;

#[expect(clippy::module_name_repetitions)]
pub struct LiveWaterLevelReporterProxy<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> {
    water_level: NonNegativeF64,
    slow_events: Vec<PackedEvent>,
    fast_events: Vec<PackedEvent>,

    local_partition: &'l mut P,
    _marker: PhantomData<(&'p (), R)>,
}

impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> fmt::Debug
    for LiveWaterLevelReporterProxy<'l, 'p, R, P>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        struct EventBufferLen(usize);

        impl fmt::Debug for EventBufferLen {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Vec<PackedEvent; {}>", self.0)
            }
        }

        fmt.debug_struct(stringify!(LiveWaterLevelReporterProxy))
            .field("water_level", &self.water_level)
            .field("slow_events", &EventBufferLen(self.slow_events.len()))
            .field("fast_events", &EventBufferLen(self.fast_events.len()))
            .finish()
    }
}

impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> Reporter
    for LiveWaterLevelReporterProxy<'l, 'p, R, P>
{
    impl_report!(speciation(&mut self, speciation: MaybeUsed<R::ReportSpeciation>) {
        if speciation.event_time < self.water_level {
            self.slow_events.push(speciation.clone().into());
        } else {
            self.fast_events.push(speciation.clone().into());
        }
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<R::ReportDispersal>) {
        if dispersal.event_time < self.water_level {
            self.slow_events.push(dispersal.clone().into());
        } else {
            self.fast_events.push(dispersal.clone().into());
        }
    });

    impl_report!(progress(&mut self, _progress: Ignored) {});
}

#[contract_trait]
impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> WaterLevelReporterProxy<'l, 'p, R, P>
    for LiveWaterLevelReporterProxy<'l, 'p, R, P>
{
    fn new(capacity: usize, local_partition: &'l mut P) -> Self {
        info!("Events will be reported using the live water-level algorithm ...");

        Self {
            water_level: NonNegativeF64::zero(),
            slow_events: Vec::with_capacity(capacity),
            fast_events: Vec::with_capacity(capacity),

            local_partition,
            _marker: PhantomData::<(&'p (), R)>,
        }
    }

    fn water_level(&self) -> NonNegativeF64 {
        self.water_level
    }

    fn advance_water_level(&mut self, water_level: NonNegativeF64) {
        // Report all events below the water level in sorted order
        self.slow_events.sort_unstable();

        for event in self.slow_events.drain(..) {
            match event.into() {
                TypedEvent::Speciation(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_speciation(&event.into());
                },
                TypedEvent::Dispersal(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_dispersal(&event.into());
                },
            }
        }

        // Update the water level
        self.water_level = water_level;

        // Move fast events below the new water level into slow events
        self.slow_events.extend(
            self.fast_events
                .extract_if(|event| event.event_time() < water_level),
        );
    }

    fn local_partition(&mut self) -> &mut P {
        self.local_partition
    }
}

impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> Drop
    for LiveWaterLevelReporterProxy<'l, 'p, R, P>
{
    fn drop(&mut self) {
        // Report all events below the water level in sorted order
        self.slow_events.sort_unstable();

        for event in self.slow_events.drain(..) {
            match event.into() {
                TypedEvent::Speciation(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_speciation(&event.into());
                },
                TypedEvent::Dispersal(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_dispersal(&event.into());
                },
            }
        }

        // Report all events above the water level in sorted order
        self.fast_events.sort_unstable();

        for event in self.fast_events.drain(..) {
            match event.into() {
                TypedEvent::Speciation(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_speciation(&event.into());
                },
                TypedEvent::Dispersal(event) => {
                    self.local_partition
                        .get_reporter()
                        .report_dispersal(&event.into());
                },
            }
        }
    }
}
