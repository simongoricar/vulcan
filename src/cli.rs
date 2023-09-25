use std::path::PathBuf;

use clap::{Parser, Subcommand, Args};

#[derive(Parser, Debug, Clone)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    #[command(name = "generate", about = "Performs the pixel sorting on an image.")]
    Generate(GenerateArgs)
}

#[derive(Args, Debug, Clone)]
pub struct GenerateArgs {
    #[arg(
        long = "input-image-path",
        help = "Path to the input image."
    )]
    pub input_image_path: PathBuf,

    #[arg(
        long = "output-image-path",
        help = "Path to the output image."
    )]
    pub output_image_path: PathBuf,
}
