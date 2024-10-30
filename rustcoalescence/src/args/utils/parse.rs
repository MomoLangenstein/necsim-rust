use anyhow::{Context, Result};
use ron::{extensions::Extensions, ser::PrettyConfig, Options};
use serde::{Deserialize, Serialize};
use serde_state::DeserializeState;

#[expect(clippy::module_name_repetitions)]
pub fn try_parse<'de, D: Deserialize<'de>>(subcommand: &str, ron_args: &'de str) -> Result<D> {
    try_parse_inner(subcommand, ron_args, |de| D::deserialize(de))
}

pub fn try_parse_state<'de, D: DeserializeState<'de, Seed>, Seed: ?Sized>(
    subcommand: &str,
    ron_args: &'de str,
    seed: &mut Seed,
) -> Result<D> {
    try_parse_inner(subcommand, ron_args, |de| D::deserialize_state(seed, de))
}

pub fn try_print<S: Serialize>(value: &S) -> Result<String> {
    ron_options()
        .to_string_pretty(value, PrettyConfig::default().struct_names(true))
        .map_err(anyhow::Error::new)
}

fn ron_options() -> Options {
    Options::default()
        .with_default_extension(Extensions::IMPLICIT_SOME)
        .with_default_extension(Extensions::UNWRAP_NEWTYPES)
        .with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
}

fn try_parse_inner<
    'de,
    D,
    F: FnOnce(
        serde_path_to_error::Deserializer<&mut ron::Deserializer<'de>>,
    ) -> Result<D, ron::error::Error>,
>(
    subcommand: &str,
    ron_args: &'de str,
    deserializer: F,
) -> Result<D> {
    let mut de_ron = ron::Deserializer::from_str_with_options(ron_args, ron_options())
        .with_context(|| {
            format!("Failed to create the {subcommand} subcommand argument parser.")
        })?;

    let mut track = serde_path_to_error::Track::new();
    let de = serde_path_to_error::Deserializer::new(&mut de_ron, &mut track);

    match deserializer(de) {
        Ok(args) => Ok(args),
        Err(err) => {
            let path = track.path();
            let err = de_ron.span_error(err);

            Err(anyhow::anyhow!(
                "{}{}{}{} @ ({}):\n{}",
                subcommand,
                if path.iter().count() >= 1 { "." } else { "" },
                path,
                if path.iter().count() >= 1 { "" } else { "*" },
                err.position,
                err.code,
            ))
        },
    }
    .with_context(|| format!("Failed to parse the {subcommand} subcommand arguments."))
}
