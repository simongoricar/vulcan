use std::{fs, path::PathBuf, sync::Arc};

use clap::Parser;
use eframe::{App, NativeOptions};
use egui::{
    CentralPanel,
    ColorImage,
    ImageData,
    SidePanel,
    TextureId,
    TextureOptions,
    Vec2,
    epaint::ImageDelta,
    load::SizedTexture,
};
use image::RgbaImage;
use miette::miette;
use tracing::{Level, trace};
use vulcan_core::sorting::{
    ImageSortingDirection,
    PixelSegmentSelectionMode,
    PixelSegmentSortDirection,
    PixelSortOptions,
    perform_pixel_sort,
};

use crate::cli::{CLIArgs, GuiArgs};

mod cli;


pub fn cmd_gui(_args: GuiArgs) -> miette::Result<()> {
    let options = NativeOptions {
        centered: true,
        vsync: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(400.0, 400.0))
            .with_active(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Vulcan",
        options,
        Box::new(|context| {
            // Enable support for displaying images.
            egui_extras::install_image_loaders(&context.egui_ctx);

            Ok(Box::new(VulcanApp::new()))
        }),
    )
    .map_err(|err| miette!("Failed to run eframe: {:?}", err))?;

    Ok(())
}

pub struct ImageRenderer {
    original_image: Option<image::DynamicImage>,
    processed_image: Option<image::DynamicImage>,
}

impl ImageRenderer {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectablePixelSortDirection {
    HorizontalAscending,
    HorizontalDescending,
    VerticalAscending,
    VerticalDescending,
}

impl SelectablePixelSortDirection {
    pub fn as_label(self) -> &'static str {
        match self {
            SelectablePixelSortDirection::HorizontalAscending => {
                "horizontal (ascending)"
            }
            SelectablePixelSortDirection::HorizontalDescending => {
                "horizontal (descending)"
            }
            SelectablePixelSortDirection::VerticalAscending => {
                "vertical (ascending)"
            }
            SelectablePixelSortDirection::VerticalDescending => {
                "vertical (descending)"
            }
        }
    }

    pub fn to_direction(self) -> ImageSortingDirection {
        match self {
            SelectablePixelSortDirection::HorizontalAscending => {
                ImageSortingDirection::Horizontal(
                    PixelSegmentSortDirection::Ascending,
                )
            }
            SelectablePixelSortDirection::HorizontalDescending => {
                ImageSortingDirection::Horizontal(
                    PixelSegmentSortDirection::Descending,
                )
            }
            SelectablePixelSortDirection::VerticalAscending => {
                ImageSortingDirection::Vertical(
                    PixelSegmentSortDirection::Ascending,
                )
            }
            SelectablePixelSortDirection::VerticalDescending => {
                ImageSortingDirection::Vertical(
                    PixelSegmentSortDirection::Descending,
                )
            }
        }
    }
}

pub struct VulcanApp {
    // TODO
    threshold_low: f32,
    threshold_high: f32,

    picked_file: Option<PathBuf>,
    opened_texture: Option<SizedTexture>,
    opened_texture_id: Option<TextureId>,
    loaded_image: Option<RgbaImage>,

    selected_direction: SelectablePixelSortDirection,
}

impl VulcanApp {
    pub fn new() -> Self {
        Self {
            threshold_low: 0.0,
            threshold_high: 1.0,
            picked_file: None,
            opened_texture: None,
            opened_texture_id: None,
            loaded_image: None,
            selected_direction:
                SelectablePixelSortDirection::HorizontalAscending,
        }
    }
}

impl App for VulcanApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        SidePanel::left("left-panel").show(ctx, |ui| {
            ui.heading("Vulcan");

            let picker_button = ui.button("Open file");
            if picker_button.clicked() {
                let optionally_picked_file = rfd::FileDialog::new().pick_file();
                if let Some(picked_file) = optionally_picked_file {
                    println!("opening file");
                    let loaded_file_bytes = fs::read(picked_file).unwrap();
                    println!("parsing file");
                    let loaded_image =
                        image::load_from_memory(&loaded_file_bytes).unwrap();
                    println!("converting file to rgba8");
                    let loaded_image_rgba8 = loaded_image.to_rgba8();

                    println!("converting to ColorImage");
                    let epaint_color_image = ColorImage::from_rgba_unmultiplied(
                        [
                            loaded_image.width() as usize,
                            loaded_image.height() as usize,
                        ],
                        loaded_image_rgba8.as_flat_samples().as_slice(),
                    );

                    println!("converting to ImageData");
                    let epaint_image_data =
                        ImageData::Color(Arc::new(epaint_color_image));

                    println!("write-locking texture manager");
                    let texture_manager = ctx.tex_manager();
                    let mut locked_texture_manager = texture_manager.write();
                    println!("allocating with texture manager");
                    let texture_id = locked_texture_manager.alloc(
                        "loaded-image".to_string(),
                        epaint_image_data,
                        TextureOptions::LINEAR,
                    );

                    println!("getting texture meta");
                    let texture_meta =
                        locked_texture_manager.meta(texture_id).unwrap();

                    println!("creating SizedTexture");
                    let sized_texture = SizedTexture::new(
                        texture_id,
                        Vec2::new(
                            texture_meta.size[0] as f32,
                            texture_meta.size[1] as f32,
                        ),
                    );

                    self.opened_texture = Some(sized_texture);
                    self.opened_texture_id = Some(texture_id);
                    self.loaded_image = Some(loaded_image_rgba8);
                }
            }

            ui.vertical(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.threshold_low, 0.0..=1.0)
                        .step_by(0.0001)
                        .min_decimals(4)
                        .max_decimals(5)
                        .drag_value_speed(0.001)
                        .text("Low threshold"),
                );
                ui.add(
                    egui::Slider::new(&mut self.threshold_high, 0.0..=1.0)
                        .step_by(0.0001)
                        .min_decimals(4)
                        .max_decimals(5)
                        .drag_value_speed(0.001)
                        .text("High threshold"),
                );
            });

            egui::ComboBox::from_label("Direction")
                .selected_text(self.selected_direction.as_label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_direction,
                        SelectablePixelSortDirection::HorizontalAscending,
                        SelectablePixelSortDirection::HorizontalAscending
                            .as_label(),
                    );
                    ui.selectable_value(
                        &mut self.selected_direction,
                        SelectablePixelSortDirection::HorizontalDescending,
                        SelectablePixelSortDirection::HorizontalDescending
                            .as_label(),
                    );
                    ui.selectable_value(
                        &mut self.selected_direction,
                        SelectablePixelSortDirection::VerticalAscending,
                        SelectablePixelSortDirection::VerticalAscending
                            .as_label(),
                    );
                    ui.selectable_value(
                        &mut self.selected_direction,
                        SelectablePixelSortDirection::VerticalDescending,
                        SelectablePixelSortDirection::VerticalDescending
                            .as_label(),
                    );
                });

            let sorting_button = ui.button("Perform pixel sorting");
            if sorting_button.clicked() {
                if let Some(loaded_image) = self.loaded_image.as_ref() {
                    let Some(texture_id) = self.opened_texture_id else {
                        panic!();
                    };

                    println!("[u] pixel sorting");
                    let sorted_image = perform_pixel_sort(
                        loaded_image.to_owned(),
                        PixelSegmentSelectionMode::LuminanceRange {
                            low: self.threshold_low,
                            high: self.threshold_high,
                        },
                        PixelSortOptions {
                            direction: self.selected_direction.to_direction(),
                        },
                    );

                    println!("[u] getting texture manager");
                    let texture_manager = ctx.tex_manager();
                    let mut locked_texture_manager = texture_manager.write();

                    println!("[u] converting to ColorImage");
                    let epaint_color_image = ColorImage::from_rgba_unmultiplied(
                        [
                            loaded_image.width() as usize,
                            loaded_image.height() as usize,
                        ],
                        sorted_image.as_flat_samples().as_slice(),
                    );

                    println!("[u] converting to ImageData");
                    let epaint_image_data =
                        ImageData::Color(Arc::new(epaint_color_image));

                    locked_texture_manager.set(
                        texture_id,
                        ImageDelta::full(
                            epaint_image_data,
                            TextureOptions::LINEAR,
                        ),
                    );

                    println!("[u] DONE");
                }
            }
        });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(sized_texture) = self.opened_texture {
                let image_widget = egui::Image::from_texture(sized_texture)
                    .max_size(Vec2::new(
                        ctx.screen_rect().width(),
                        ctx.screen_rect().height(),
                    ));

                ui.add(image_widget);
            }
        });
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
