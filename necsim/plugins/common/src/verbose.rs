use std::fmt;

use necsim_core::{impl_report, reporter::Reporter};

#[expect(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct VerboseReporter(());

impl fmt::Debug for VerboseReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(VerboseReporter)).finish()
    }
}

impl serde::Serialize for VerboseReporter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for VerboseReporter {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::default())
    }
}

impl Reporter for VerboseReporter {
    impl_report!(speciation(&mut self, speciation: Used) {
        info!("{:#?}", speciation);
    });

    impl_report!(dispersal(&mut self, dispersal: Used) {
        info!("{:#?}", dispersal);
    });

    impl_report!(progress(&mut self, remaining: Used) {
        info!("Remaining: {}", remaining);
    });
}
