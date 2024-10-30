use crate::{impl_report, reporter::Reporter};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct NullReporter;

impl Reporter for NullReporter {
    impl_report!(speciation(&mut self, _speciation: Ignored) {});

    impl_report!(dispersal(&mut self, _dispersal: Ignored) {});

    impl_report!(progress(&mut self, _progress: Ignored) {});
}
