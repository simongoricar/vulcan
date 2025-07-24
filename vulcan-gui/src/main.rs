use clap::Parser;
use eframe::NativeOptions;
use egui::{FontDefinitions, Vec2};
use egui_phosphor::Variant;
use miette::miette;
use tracing::{Level, trace};

use crate::{
    cli::{CLIArgs, GuiArgs},
    gui::VulcanGui,
    worker::WorkerHandle,
};

mod cancellation;
mod cli;
mod gui;
mod worker;


const EGUI_APP_ID: &str = "org.simongoricar.vulcan";


pub fn cmd_gui(_args: GuiArgs) -> miette::Result<()> {
    let worker = WorkerHandle::initialize();

    let options = NativeOptions {
        centered: true,
        vsync: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1200.0, 800.0))
            .with_active(true)
            .with_clamp_size_to_monitor_size(true)
            .with_app_id(EGUI_APP_ID)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Vulcan",
        options,
        Box::new(move |context| {
            // Enable support for displaying images.
            egui_extras::install_image_loaders(&context.egui_ctx);

            // Enables Phosphor icons.
            let mut fonts = FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, Variant::Regular);
            context.egui_ctx.set_fonts(fonts);

            Ok(Box::new(VulcanGui::new(worker)))
        }),
    )
    .map_err(|err| miette!("Failed to run eframe: {:?}", err))?;

    Ok(())
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
