use serde::{de::IgnoredAny, Deserialize};

use crate::args::utils::parse::try_parse;

pub fn parse_and_normalise(ron_args: &str) -> anyhow::Result<()> {
    // Check for the overall config stucture
    //  (1) are all required fields defined
    //  (2) are any unknown fields defined
    let SimulateArgsFields { .. } = try_parse("simulate", ron_args)?;

    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Simulate")]
#[expect(dead_code)]
struct SimulateArgsFields {
    #[serde(alias = "speciation_probability_per_generation")]
    speciation: IgnoredAny,

    #[serde(default)]
    sample: IgnoredAny,

    #[serde(default)]
    pause: Option<IgnoredAny>,

    #[serde(alias = "randomness")]
    #[serde(default)]
    rng: IgnoredAny,

    scenario: IgnoredAny,

    algorithm: IgnoredAny,

    #[serde(default)]
    partitioning: IgnoredAny,

    #[serde(alias = "event_log")]
    #[serde(default)]
    log: Option<IgnoredAny>,

    reporters: Vec<IgnoredAny>,
}
