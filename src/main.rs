use std::path::PathBuf;

use clap::Parser;
use cli::CLIArgs;
use tracing::{trace, Level};

use crate::generation::cmd_generate;

mod cli;
mod generation;

pub trait ExtendablePath {
    fn with_suffix_to_stem<S>(&self, suffix: S) -> Option<Self>
    where
        S: AsRef<str>,
        Self: Sized;
}

impl ExtendablePath for PathBuf {
    fn with_suffix_to_stem<S>(&self, suffix: S) -> Option<Self>
    where
        S: AsRef<str>,
        Self: Sized,
    {
        let file_stem = match self.file_stem() {
            Some(stem) => stem.to_string_lossy(),
            None => return None,
        };

        let file_extension = self.extension();

        Some(self.with_file_name(format!(
            "{stem}{suffix}{extension}",
            stem = file_stem,
            suffix = suffix.as_ref(),
            extension = match file_extension {
                Some(extension) => format!(".{}", extension.to_string_lossy()),
                None => String::from(""),
            }
        )))
    }
}


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
        cli::Command::Generate(generate_args) => {
            trace!("Calling the generation function.");
            cmd_generate(generate_args)?;
        }
    };

    Ok(())
}
