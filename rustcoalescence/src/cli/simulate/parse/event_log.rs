use serde::{Deserialize, Deserializer};
use serde_state::DeserializeState;

use necsim_impls_std::event_log::recorder::EventLogConfig;

use crate::args::{
    config::{
        partitioning::Partitioning,
        pause::Pause,
        sample::{Sample, SampleMode},
    },
    utils::parse::try_parse_state,
};

use super::super::BufferingSimulateArgsBuilder;

pub(in super::super) fn parse_and_normalise(
    ron_args: &str,
    normalised_args: &mut BufferingSimulateArgsBuilder,
    partitioning: &Partitioning,
    sample: &Sample,
    pause: &Option<Pause>,
) -> anyhow::Result<Option<EventLogConfig>> {
    let mut event_log_check = partitioning.get_event_log_check();
    if event_log_check.0.is_ok() && (pause.is_some() || !matches!(sample.mode, SampleMode::Genesis))
    {
        event_log_check.0 = Err(anyhow::anyhow!(
            "Pausing or resuming a simulation requires an event log"
        ));
    }

    let SimulateArgsEventLogOnly { event_log } =
        try_parse_state("simulate", ron_args, &mut event_log_check)?;

    normalised_args.log(&event_log);

    let event_log = match event_log {
        Some(event_log)
            if event_log
                .directory()
                .ends_with("I-solemnly-swear-that-I-am-up-to-no-good") =>
        {
            None
        },
        event_log => event_log,
    };

    Ok(event_log)
}

struct SimulateArgsEventLogOnly {
    event_log: Option<EventLogConfig>,
}

impl<'de> DeserializeState<'de, (anyhow::Result<()>, anyhow::Result<()>)>
    for SimulateArgsEventLogOnly
{
    fn deserialize_state<D>(
        event_log_check: &mut (anyhow::Result<()>, anyhow::Result<()>),
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let raw = SimulateArgsEventLogOnlyRaw::deserialize_state(event_log_check, deserializer)?;

        if raw.event_log.is_none() {
            event_log_check
                .0
                .as_ref()
                .map_err(serde::de::Error::custom)?;
        } else {
            event_log_check
                .1
                .as_ref()
                .map_err(serde::de::Error::custom)?;
        }

        Ok(SimulateArgsEventLogOnly {
            event_log: raw.event_log,
        })
    }
}

#[derive(DeserializeState)]
#[serde(deserialize_state = "(anyhow::Result<()>, anyhow::Result<()>)")]
#[serde(rename = "Simulate")]
struct SimulateArgsEventLogOnlyRaw {
    #[serde(alias = "log")]
    #[serde(default)]
    #[serde(deserialize_state_with = "deserialize_state_event_log")]
    event_log: Option<EventLogConfig>,
}

fn deserialize_state_event_log<'de, D: Deserializer<'de>>(
    event_log_check: &mut (anyhow::Result<()>, anyhow::Result<()>),
    deserializer: D,
) -> Result<Option<EventLogConfig>, D::Error> {
    let maybe_event_log = <Option<EventLogConfig>>::deserialize(deserializer)?;

    if maybe_event_log.is_none() {
        event_log_check
            .0
            .as_ref()
            .map_err(serde::de::Error::custom)?;
    } else {
        event_log_check
            .1
            .as_ref()
            .map_err(serde::de::Error::custom)?;
    }

    Ok(maybe_event_log)
}
