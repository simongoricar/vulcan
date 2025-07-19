use std::{fs, path::PathBuf, sync::Arc};

use eframe::{App, NativeOptions};
use egui::{epaint::TextureManager, load::SizedTexture, CentralPanel, ColorImage, ImageData, ImageSource, TextureHandle, TextureId, TextureOptions, Vec2};
use miette::miette;

use crate::cli::GuiArgs;

pub fn cmd_gui(_args: GuiArgs) -> miette::Result<()> {
    let options = NativeOptions {
        centered: true,
        vsync: true,
        viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(400.0, 400.0)).with_active(true).with_drag_and_drop(true),
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


pub struct VulcanApp {
    // TODO

    picked_file: Option<PathBuf>,
    opened_texture: Option<SizedTexture>,
    opened_texture_id: Option<TextureId>
}

impl VulcanApp {
    pub fn new() -> Self {
        Self {
            picked_file: None,
            opened_texture: None,
            opened_texture_id: None
        }
    }
}

impl App for VulcanApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Vulcan");

            let picker_button = ui.button("Open file");
            if picker_button.clicked() {
                let optionally_picked_file = rfd::FileDialog::new().pick_file();
                if let Some(picked_file) = optionally_picked_file {
                    println!("opening file");
                    let loaded_file_bytes = fs::read(picked_file).unwrap();
                    println!("parsing file");
                    let loaded_image = image::load_from_memory(&loaded_file_bytes).unwrap();
                    println!("converting file to rgba8");
                    let loaded_image_rgba8 = loaded_image.to_rgba8();

                    println!("converting to ColorImage");
                    let epaint_color_image = ColorImage::from_rgba_unmultiplied(
                        [loaded_image.width() as usize, loaded_image.height() as usize],
                        loaded_image_rgba8.as_flat_samples().as_slice()
                    );

                    println!("converting to ImageData");
                    let epaint_image_data = ImageData::Color(Arc::new(epaint_color_image));
                    
                    println!("write-locking texture manager");
                    let texture_manager = ctx.tex_manager();
                    let mut locked_texture_manager = texture_manager.write();
                    println!("allocating with texture manager");
                    let texture_id = locked_texture_manager.alloc("loaded-image".to_string(), epaint_image_data, TextureOptions::LINEAR);

                    println!("getting texture meta");
                    let texture_meta = locked_texture_manager.meta(texture_id).unwrap();

                    println!("creating SizedTexture");
                    let sized_texture = SizedTexture::new(texture_id, Vec2::new(texture_meta.size[0] as f32, texture_meta.size[1] as f32));

                    self.opened_texture = Some(sized_texture);
                    self.opened_texture_id = Some(texture_id);
                }
            }

            if let Some(sized_texture) = self.opened_texture {
                let image_widget = egui::Image::from_texture(sized_texture).max_size(Vec2::new(ctx.screen_rect().width(), ctx.screen_rect().height()));

                ui.add(image_widget);
            }
        });
    }
}
