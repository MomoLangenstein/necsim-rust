use anyhow::{Context, Result};
use log::LevelFilter;

use necsim_core::{event::TypedEvent, reporter::Reporter};

use necsim_plugins_core::match_any_reporter_plugin_vec;

use crate::args::{
    cli::{replay::ReplayArgs, CommandArgs},
    utils::parse::{try_parse, try_print},
};

#[expect(clippy::module_name_repetitions)]
pub fn replay_with_logger(replay_args: CommandArgs) -> Result<()> {
    log::set_max_level(LevelFilter::Info);

    let replay_args: ReplayArgs = try_parse("replay", &replay_args.into_config_string())?;

    let config_str =
        try_print(&replay_args).context("Failed to normalise the event replay config.")?;

    println!("\n{:=^80}\n", " Replay Configuration ");
    println!("{}", config_str.trim_start_matches("Replay"));
    println!("\n{:=^80}\n", " Replay Configuration ");

    info!("Starting event replay ...");

    match_any_reporter_plugin_vec!(replay_args.reporters => |mut reporter| {
        reporter.initialise().map_err(anyhow::Error::msg)?;

        let mut remaining = replay_args.event_log.length() as u64;

        reporter.report_progress(&remaining.into());

        for event in replay_args.event_log {
            remaining -= 1;
            reporter.report_progress(&remaining.into());

            match event.into() {
                TypedEvent::Speciation(event) => {
                    reporter.report_speciation(&event.into());
                },
                TypedEvent::Dispersal(event) => {
                    reporter.report_dispersal(&event.into());
                },
            }
        }

        if log::log_enabled!(log::Level::Info) {
            println!("\n");
            println!("{:=^80}", " Reporter Summary ");
            println!();
        }
        reporter.finalise();
        if log::log_enabled!(log::Level::Info) {
            println!();
            println!("{:=^80}", " Reporter Summary ");
            println!();
        }
    });

    info!("The event replay has completed.");

    Ok(())
}
