use core::{fmt, marker::PhantomData};
use necsim_core_bond::NonNegativeF64;

use necsim_core::{impl_report, reporter::Reporter};

use necsim_partitioning_core::LocalPartition;

use super::WaterLevelReporterProxy;

#[expect(clippy::module_name_repetitions)]
pub struct RecordedWaterLevelReporterProxy<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> {
    water_level: NonNegativeF64,

    local_partition: &'l mut P,
    _marker: PhantomData<(&'p (), R)>,
}

impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> fmt::Debug
    for RecordedWaterLevelReporterProxy<'l, 'p, R, P>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(RecordedWaterLevelReporterProxy))
            .field("water_level", &self.water_level)
            .finish()
    }
}

impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> Reporter
    for RecordedWaterLevelReporterProxy<'l, 'p, R, P>
{
    impl_report!(speciation(&mut self, speciation: MaybeUsed<R::ReportSpeciation>) {
        self.local_partition.get_reporter().report_speciation(speciation.into());
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<R::ReportDispersal>) {
        self.local_partition.get_reporter().report_dispersal(dispersal.into());
    });

    impl_report!(progress(&mut self, _progress: Ignored) {});
}

#[contract_trait]
impl<'l, 'p, R: Reporter, P: LocalPartition<'p, R>> WaterLevelReporterProxy<'l, 'p, R, P>
    for RecordedWaterLevelReporterProxy<'l, 'p, R, P>
{
    fn new(_capacity: usize, local_partition: &'l mut P) -> Self {
        info!("Events will be reported using the recorded water-level algorithm ...");

        Self {
            water_level: NonNegativeF64::zero(),

            local_partition,
            _marker: PhantomData::<(&'p (), R)>,
        }
    }

    fn water_level(&self) -> NonNegativeF64 {
        self.water_level
    }

    fn advance_water_level(&mut self, water_level: NonNegativeF64) {
        self.water_level = water_level;
    }

    fn local_partition(&mut self) -> &mut P {
        self.local_partition
    }
}
