use serde::{Deserialize, Deserializer, Serialize};

use necsim_impls_std::event_log::replay::EventLogReplay;

use necsim_plugins_core::import::{AnyReporterPluginVec, ReporterPluginLibrary};

#[derive(Serialize, Debug)]
#[serde(rename = "Replay")]
#[expect(clippy::module_name_repetitions)]
pub struct ReplayArgs {
    #[serde(rename = "log", alias = "event_log")]
    pub event_log: EventLogReplay,
    pub mode: ReplayMode,
    pub reporters: AnyReporterPluginVec,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[expect(clippy::module_name_repetitions)]
#[serde(deny_unknown_fields)]
pub enum ReplayMode {
    Strict,
    WarnOnly,
}

impl Default for ReplayMode {
    fn default() -> Self {
        Self::Strict
    }
}

impl<'de> Deserialize<'de> for ReplayArgs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ReplayArgsRaw::deserialize(deserializer)?;

        let event_log = raw.event_log;
        let mode = raw.mode;
        let reporters = raw.reporters.into_iter().flatten().collect();

        let (report_speciation, report_dispersal) = match &reporters {
            AnyReporterPluginVec::IgnoreSpeciationIgnoreDispersalIgnoreProgress(..)
            | AnyReporterPluginVec::IgnoreSpeciationIgnoreDispersalReportProgress(..) => {
                (false, false)
            },
            AnyReporterPluginVec::IgnoreSpeciationReportDispersalIgnoreProgress(..)
            | AnyReporterPluginVec::IgnoreSpeciationReportDispersalReportProgress(..) => {
                (false, true)
            },
            AnyReporterPluginVec::ReportSpeciationIgnoreDispersalIgnoreProgress(..)
            | AnyReporterPluginVec::ReportSpeciationIgnoreDispersalReportProgress(..) => {
                (true, false)
            },
            AnyReporterPluginVec::ReportSpeciationReportDispersalIgnoreProgress(..)
            | AnyReporterPluginVec::ReportSpeciationReportDispersalReportProgress(..) => {
                (true, true)
            },
        };

        let valid = if report_speciation
            && !event_log.with_speciation()
            && report_dispersal
            && !event_log.with_dispersal()
        {
            Err(
                "The reporters require speciation and dispersal events, but the event log cannot \
                 provide either.",
            )
        } else if report_speciation && !event_log.with_speciation() {
            Err("The reporters require speciation events, but the event log cannot provide them.")
        } else if report_dispersal && !event_log.with_dispersal() {
            Err("The reporters require dispersal events, but the event log cannot provide them.")
        } else {
            Ok(())
        };

        match (valid, mode) {
            (Ok(()), _) => Ok(Self {
                event_log,
                mode,
                reporters,
            }),
            (Err(error), ReplayMode::WarnOnly) => {
                warn!("{}", error);

                Ok(Self {
                    event_log,
                    mode,
                    reporters,
                })
            },
            (Err(error), ReplayMode::Strict) => Err(serde::de::Error::custom(error)),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Replay")]
struct ReplayArgsRaw {
    #[serde(alias = "log")]
    event_log: EventLogReplay,
    #[serde(default)]
    mode: ReplayMode,
    reporters: Vec<ReporterPluginLibrary>,
}
