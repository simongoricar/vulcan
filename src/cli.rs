use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // #[command(
    //     name = "generate",
    //     about = "Performs the pixel sorting on an image."
    // )]
    // Generate(GenerateArgs),
    #[command(name = "gui", about = "Open the vulcan pixel sorting GUI.")]
    Gui(GuiArgs),
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct GenerateArgs {
    #[arg(long = "input-image-path", help = "Path to the input image.")]
    pub input_image_path: PathBuf,

    #[arg(long = "output-image-path", help = "Path to the output image.")]
    pub output_image_path: PathBuf,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct GuiArgs {
    // TODO
}
