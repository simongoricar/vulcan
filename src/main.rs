use std::path::PathBuf;

use clap::Parser;
use cli::CLIArgs;
use tracing::{Level, trace};

use crate::gui::cmd_gui;

mod cli;
mod generation;
mod gui;

fn initialize_tracing() {
    let fmt_subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(fmt_subscriber)
        .expect("failed to set global tracing subscriber");
}

fn main() -> miette::Result<()> {
    initialize_tracing();

    let args = CLIArgs::parse();

    match args.command {
        // cli::Command::Generate(generate_args) => {
        //     trace!("Calling the generation function.");
        //     cmd_generate(generate_args)?;
        // }
        cli::Command::Gui(gui_args) => {
            trace!("Calling the GUI function.");
            cmd_gui(gui_args)?;
        }
    };

    Ok(())
}
