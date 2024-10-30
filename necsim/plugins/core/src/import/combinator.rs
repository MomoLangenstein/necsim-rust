use std::{
    fmt,
    iter::{FromIterator, IntoIterator},
    marker::PhantomData,
    path::Path,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use necsim_core::{
    impl_finalise, impl_report,
    reporter::{
        boolean::{Boolean, False, True},
        Reporter,
    },
};

use crate::{export::Reporters, import::ReporterPlugin};

use super::ReporterPluginLibrary;

pub struct ReporterPluginVec<
    ReportSpeciation: Boolean,
    ReportDispersal: Boolean,
    ReportProgress: Boolean,
> {
    plugins: Box<[ReporterPlugin]>,
    marker: PhantomData<(ReportSpeciation, ReportDispersal, ReportProgress)>,
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean> fmt::Debug
    for ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(stringify!(ReporterPluginVec))
            .field("plugins", &self.plugins)
            .finish()
    }
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean>
    ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>
{
    #[must_use]
    pub fn internal_filter<
        KeepSpeciation: Boolean,
        KeepDispersal: Boolean,
        KeepProgress: Boolean,
    >(
        self,
    ) -> Self {
        let mut plugins: Vec<ReporterPlugin> = self.plugins.into_vec();

        plugins.retain(|plugin| {
            (plugin.filter.report_speciation && KeepSpeciation::VALUE)
                || (plugin.filter.report_dispersal && KeepDispersal::VALUE)
                || (plugin.filter.report_progress && KeepProgress::VALUE)
        });

        Self {
            plugins: plugins.into_boxed_slice(),
            marker: self.marker,
        }
    }

    pub fn with_lifetime<Q, F: FnOnce(Self) -> Q>(self, inner: F) -> Q {
        let libraries = self
            .plugins
            .iter()
            .map(|plugin| &plugin.library)
            .cloned()
            .collect::<Vec<_>>();

        let result = inner(self);

        std::mem::drop(libraries);

        result
    }
}

impl<ReportSpeciation: Boolean, ReportDispersal: Boolean, ReportProgress: Boolean> Reporter
    for ReporterPluginVec<ReportSpeciation, ReportDispersal, ReportProgress>
{
    impl_report!(speciation(&mut self, speciation: MaybeUsed<ReportSpeciation>) {
        for plugin in &mut self.plugins {
            if plugin.filter.report_speciation {
                plugin.reporter.report_speciation(speciation.into());
            }
        }
    });

    impl_report!(dispersal(&mut self, dispersal: MaybeUsed<ReportDispersal>) {
        for plugin in &mut self.plugins {
            if plugin.filter.report_dispersal {
                plugin.reporter.report_dispersal(dispersal.into());
            }
        }
    });

    impl_report!(progress(&mut self, progress: MaybeUsed<ReportProgress>) {
        for plugin in &mut self.plugins {
            if plugin.filter.report_progress {
                plugin.reporter.report_progress(progress.into());
            }
        }
    });

    impl_finalise!((self) {
        for plugin in self.plugins.into_vec() {
            plugin.finalise();
        }
    });

    fn initialise(&mut self) -> Result<(), String> {
        for plugin in &mut self.plugins {
            plugin.reporter.initialise()?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum AnyReporterPluginVec {
    IgnoreSpeciationIgnoreDispersalIgnoreProgress(ReporterPluginVec<False, False, False>),
    IgnoreSpeciationIgnoreDispersalReportProgress(ReporterPluginVec<False, False, True>),
    IgnoreSpeciationReportDispersalIgnoreProgress(ReporterPluginVec<False, True, False>),
    IgnoreSpeciationReportDispersalReportProgress(ReporterPluginVec<False, True, True>),
    ReportSpeciationIgnoreDispersalIgnoreProgress(ReporterPluginVec<True, False, False>),
    ReportSpeciationIgnoreDispersalReportProgress(ReporterPluginVec<True, False, True>),
    ReportSpeciationReportDispersalIgnoreProgress(ReporterPluginVec<True, True, False>),
    ReportSpeciationReportDispersalReportProgress(ReporterPluginVec<True, True, True>),
}

impl FromIterator<ReporterPlugin> for AnyReporterPluginVec {
    fn from_iter<I: IntoIterator<Item = ReporterPlugin>>(iter: I) -> Self {
        let plugins = iter
            .into_iter()
            .collect::<Vec<ReporterPlugin>>()
            .into_boxed_slice();

        let report_speciation = plugins
            .iter()
            .any(|reporter| reporter.filter.report_speciation);
        let report_dispersal = plugins
            .iter()
            .any(|reporter| reporter.filter.report_dispersal);
        let report_progress = plugins
            .iter()
            .any(|reporter| reporter.filter.report_progress);

        match (report_speciation, report_dispersal, report_progress) {
            (false, false, false) => {
                Self::IgnoreSpeciationIgnoreDispersalIgnoreProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(False, False, False)>,
                })
            },
            (false, false, true) => {
                Self::IgnoreSpeciationIgnoreDispersalReportProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(False, False, True)>,
                })
            },
            (false, true, false) => {
                Self::IgnoreSpeciationReportDispersalIgnoreProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(False, True, False)>,
                })
            },
            (false, true, true) => {
                Self::IgnoreSpeciationReportDispersalReportProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(False, True, True)>,
                })
            },
            (true, false, false) => {
                Self::ReportSpeciationIgnoreDispersalIgnoreProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(True, False, False)>,
                })
            },
            (true, false, true) => {
                Self::ReportSpeciationIgnoreDispersalReportProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(True, False, True)>,
                })
            },
            (true, true, false) => {
                Self::ReportSpeciationReportDispersalIgnoreProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(True, True, False)>,
                })
            },
            (true, true, true) => {
                Self::ReportSpeciationReportDispersalReportProgress(ReporterPluginVec {
                    plugins,
                    marker: PhantomData::<(True, True, True)>,
                })
            },
        }
    }
}

#[macro_export]
macro_rules! match_any_reporter_plugin_vec {
    ($any:expr => | $inner:ident | $code:block) => {{
        use $crate::import::AnyReporterPluginVec::*;

        match $any {
            IgnoreSpeciationIgnoreDispersalIgnoreProgress($inner) => $code,
            IgnoreSpeciationIgnoreDispersalReportProgress($inner) => $code,
            IgnoreSpeciationReportDispersalIgnoreProgress($inner) => $code,
            IgnoreSpeciationReportDispersalReportProgress($inner) => $code,
            ReportSpeciationIgnoreDispersalIgnoreProgress($inner) => $code,
            ReportSpeciationIgnoreDispersalReportProgress($inner) => $code,
            ReportSpeciationReportDispersalIgnoreProgress($inner) => $code,
            ReportSpeciationReportDispersalReportProgress($inner) => $code,
        }
    }};
    ($any:expr => | mut $inner:ident | $code:block) => {{
        use $crate::import::AnyReporterPluginVec::*;

        match $any {
            IgnoreSpeciationIgnoreDispersalIgnoreProgress(mut $inner) => $code,
            IgnoreSpeciationIgnoreDispersalReportProgress(mut $inner) => $code,
            IgnoreSpeciationReportDispersalIgnoreProgress(mut $inner) => $code,
            IgnoreSpeciationReportDispersalReportProgress(mut $inner) => $code,
            ReportSpeciationIgnoreDispersalIgnoreProgress(mut $inner) => $code,
            ReportSpeciationIgnoreDispersalReportProgress(mut $inner) => $code,
            ReportSpeciationReportDispersalIgnoreProgress(mut $inner) => $code,
            ReportSpeciationReportDispersalReportProgress(mut $inner) => $code,
        }
    }};
}

impl AnyReporterPluginVec {
    pub fn with_lifetime<Q, F: FnOnce(Self) -> Q>(self, inner: F) -> Q {
        match self {
            Self::IgnoreSpeciationIgnoreDispersalIgnoreProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::IgnoreSpeciationIgnoreDispersalIgnoreProgress(vec))
            }),
            Self::IgnoreSpeciationIgnoreDispersalReportProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::IgnoreSpeciationIgnoreDispersalReportProgress(vec))
            }),
            Self::IgnoreSpeciationReportDispersalIgnoreProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::IgnoreSpeciationReportDispersalIgnoreProgress(vec))
            }),
            Self::IgnoreSpeciationReportDispersalReportProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::IgnoreSpeciationReportDispersalReportProgress(vec))
            }),
            Self::ReportSpeciationIgnoreDispersalIgnoreProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::ReportSpeciationIgnoreDispersalIgnoreProgress(vec))
            }),
            Self::ReportSpeciationIgnoreDispersalReportProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::ReportSpeciationIgnoreDispersalReportProgress(vec))
            }),
            Self::ReportSpeciationReportDispersalIgnoreProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::ReportSpeciationReportDispersalIgnoreProgress(vec))
            }),
            Self::ReportSpeciationReportDispersalReportProgress(vec) => vec.with_lifetime(|vec| {
                inner(Self::ReportSpeciationReportDispersalReportProgress(vec))
            }),
        }
    }
}

impl<'de> Deserialize<'de> for AnyReporterPluginVec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let plugins = Vec::<ReporterPluginLibrary>::deserialize(deserializer)?;

        Ok(plugins.into_iter().flatten().collect())
    }
}

impl Serialize for AnyReporterPluginVec {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Plugin<'r> {
            library: &'r Path,
            reporters: Vec<Reporters<'r>>,
        }

        let plugins = match_any_reporter_plugin_vec! { self => |vec| { &*vec.plugins } };

        let mut previous_library = None;
        let mut previous_reporters = Vec::new();

        let mut plugin_libraries = Vec::new();

        for reporter_plugin in plugins {
            if let Some(previous_library) = previous_library {
                if previous_library != reporter_plugin.library.path {
                    plugin_libraries.push(Plugin {
                        library: previous_library,
                        reporters: std::mem::take(&mut previous_reporters),
                    });
                }
            }

            previous_library = Some(&reporter_plugin.library.path);

            previous_reporters.push(Reporters::DynReporter(&**reporter_plugin.reporter));
        }

        if let Some(previous_library) = previous_library {
            plugin_libraries.push(Plugin {
                library: previous_library,
                reporters: previous_reporters,
            });
        }

        plugin_libraries.serialize(serializer)
    }
}
