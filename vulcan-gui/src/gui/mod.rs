use std::{path::PathBuf, sync::Arc};

use eframe::App;
use egui::{
    Align2,
    CentralPanel,
    ColorImage,
    Direction,
    ImageData,
    Pos2,
    TextureOptions,
    Vec2,
    epaint::TextureManager,
    load::SizedTexture,
    mutex::RwLock,
};
use egui_taffy::{TuiBuilderLogic, taffy};
use image::RgbaImage;
use vulcan_core::io::ImageSaveError;

use crate::{
    gui::panels::{center::CentralView, right::RightSidebar},
    worker::{ImageLoadError, WorkerHandle, WorkerResponse},
};

mod panels;



pub struct SourceImage {
    file_path: PathBuf,
    image: Arc<RgbaImage>,
    image_aspect_ratio: f32,
    image_texture: SizedTexture,
}

pub struct ProcessedImage {
    image: Arc<RgbaImage>,
    image_aspect_ratio: f32,
    image_texture: SizedTexture,
}


pub struct SharedState {
    source_image: Option<SourceImage>,
    processed_image: Option<ProcessedImage>,
    is_loading_image: bool,
    is_processing_image: bool,
    is_saving_image: bool,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            source_image: None,
            processed_image: None,
            is_loading_image: false,
            is_processing_image: false,
            is_saving_image: false,
        }
    }
}



fn allocate_texture_for_rgba8_image(
    image: &RgbaImage,
    texture_manager: &RwLock<TextureManager>,
) -> SizedTexture {
    let epaint_color_image = ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_flat_samples().as_slice(),
    );

    let epaint_image_data = ImageData::Color(Arc::new(epaint_color_image));


    let mut locked_texture_manager = texture_manager.write();

    let texture_id = locked_texture_manager.alloc(
        "vulcan-image".to_string(),
        epaint_image_data,
        TextureOptions::LINEAR,
    );

    let texture_meta = locked_texture_manager.meta(texture_id)
        // PANIC SAFETY: This can never panic, as we just allocated the texture.
        .expect("texture should be allocated at this point");

    SizedTexture::new(
        texture_id,
        Vec2::new(
            texture_meta.size[0] as f32,
            texture_meta.size[1] as f32,
        ),
    )
}



pub struct VulcanGui {
    state: SharedState,

    worker: WorkerHandle,

    central_view: CentralView,
    right_sidebar: RightSidebar,
    // TODO
    // threshold_low: f32,
    // threshold_high: f32,

    // opened_texture: Option<SizedTexture>,
    // opened_texture_id: Option<TextureId>,
    // loaded_image: Option<RgbaImage>,

    // selected_direction: SelectablePixelSortDirection,
}

impl VulcanGui {
    pub fn new(worker: WorkerHandle) -> Self {
        Self {
            state: SharedState::new(),
            worker,
            central_view: CentralView::new(),
            right_sidebar: RightSidebar::new(),
            // threshold_low: 0.0,
            // threshold_high: 1.0,
            // opened_texture: None,
            // opened_texture_id: None,
            // loaded_image: None,
            // selected_direction:
            //     SelectablePixelSortDirection::HorizontalAscending,
        }
    }
}

impl App for VulcanGui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // SidePanel::left("left-panel").show(ctx, |ui| {
        //     ui.heading("Vulcan");

        //     let picker_button = ui.button("Open file");
        //     if picker_button.clicked() {
        //         let optionally_picked_file = rfd::FileDialog::new().pick_file();
        //         if let Some(picked_file) = optionally_picked_file {
        //             println!("opening file");
        //             let loaded_file_bytes = fs::read(picked_file).unwrap();
        //             println!("parsing file");
        //             let loaded_image =
        //                 image::load_from_memory(&loaded_file_bytes).unwrap();
        //             println!("converting file to rgba8");
        //             let loaded_image_rgba8 = loaded_image.to_rgba8();

        //             println!("converting to ColorImage");
        //             let epaint_color_image = ColorImage::from_rgba_unmultiplied(
        //                 [
        //                     loaded_image.width() as usize,
        //                     loaded_image.height() as usize,
        //                 ],
        //                 loaded_image_rgba8.as_flat_samples().as_slice(),
        //             );

        //             println!("converting to ImageData");
        //             let epaint_image_data =
        //                 ImageData::Color(Arc::new(epaint_color_image));

        //             println!("write-locking texture manager");
        //             let texture_manager = ctx.tex_manager();
        //             let mut locked_texture_manager = texture_manager.write();
        //             println!("allocating with texture manager");
        //             let texture_id = locked_texture_manager.alloc(
        //                 "loaded-image".to_string(),
        //                 epaint_image_data,
        //                 TextureOptions::LINEAR,
        //             );

        //             println!("getting texture meta");
        //             let texture_meta =
        //                 locked_texture_manager.meta(texture_id).unwrap();

        //             println!("creating SizedTexture");
        //             let sized_texture = SizedTexture::new(
        //                 texture_id,
        //                 Vec2::new(
        //                     texture_meta.size[0] as f32,
        //                     texture_meta.size[1] as f32,
        //                 ),
        //             );

        //             self.opened_texture = Some(sized_texture);
        //             self.opened_texture_id = Some(texture_id);
        //             self.loaded_image = Some(loaded_image_rgba8);
        //         }
        //     }

        //     ui.vertical(|ui| {
        //         ui.add(
        //             egui::Slider::new(&mut self.threshold_low, 0.0..=1.0)
        //                 .step_by(0.0001)
        //                 .min_decimals(4)
        //                 .max_decimals(5)
        //                 .drag_value_speed(0.001)
        //                 .text("Low threshold"),
        //         );
        //         ui.add(
        //             egui::Slider::new(&mut self.threshold_high, 0.0..=1.0)
        //                 .step_by(0.0001)
        //                 .min_decimals(4)
        //                 .max_decimals(5)
        //                 .drag_value_speed(0.001)
        //                 .text("High threshold"),
        //         );
        //     });

        //     egui::ComboBox::from_label("Direction")
        //         .selected_text(self.selected_direction.as_label())
        //         .show_ui(ui, |ui| {
        //             ui.selectable_value(
        //                 &mut self.selected_direction,
        //                 SelectablePixelSortDirection::HorizontalAscending,
        //                 SelectablePixelSortDirection::HorizontalAscending
        //                     .as_label(),
        //             );
        //             ui.selectable_value(
        //                 &mut self.selected_direction,
        //                 SelectablePixelSortDirection::HorizontalDescending,
        //                 SelectablePixelSortDirection::HorizontalDescending
        //                     .as_label(),
        //             );
        //             ui.selectable_value(
        //                 &mut self.selected_direction,
        //                 SelectablePixelSortDirection::VerticalAscending,
        //                 SelectablePixelSortDirection::VerticalAscending
        //                     .as_label(),
        //             );
        //             ui.selectable_value(
        //                 &mut self.selected_direction,
        //                 SelectablePixelSortDirection::VerticalDescending,
        //                 SelectablePixelSortDirection::VerticalDescending
        //                     .as_label(),
        //             );
        //         });

        //     let sorting_button = ui.button("Perform pixel sorting");
        //     if sorting_button.clicked() {
        //         if let Some(loaded_image) = self.loaded_image.as_ref() {
        //             let Some(texture_id) = self.opened_texture_id else {
        //                 panic!();
        //             };

        //             println!("[u] pixel sorting");
        //             let sorted_image = perform_pixel_sort(
        //                 loaded_image.to_owned(),
        //                 PixelSegmentSelectionMode::LuminanceRange {
        //                     low: self.threshold_low,
        //                     high: self.threshold_high,
        //                 },
        //                 PixelSortOptions {
        //                     direction: self
        //                         .selected_direction
        //                         .to_image_sorting_direction(),
        //                 },
        //             );

        //             println!("[u] getting texture manager");
        //             let texture_manager = ctx.tex_manager();
        //             let mut locked_texture_manager = texture_manager.write();

        //             println!("[u] converting to ColorImage");
        //             let epaint_color_image = ColorImage::from_rgba_unmultiplied(
        //                 [
        //                     loaded_image.width() as usize,
        //                     loaded_image.height() as usize,
        //                 ],
        //                 sorted_image.as_flat_samples().as_slice(),
        //             );

        //             println!("[u] converting to ImageData");
        //             let epaint_image_data =
        //                 ImageData::Color(Arc::new(epaint_color_image));

        //             locked_texture_manager.set(
        //                 texture_id,
        //                 ImageDelta::full(
        //                     epaint_image_data,
        //                     TextureOptions::LINEAR,
        //                 ),
        //             );

        //             println!("[u] DONE");
        //         }
        //     }
        // });


        let mut toasts = egui_toast::Toasts::new()
            .anchor(Align2::LEFT_TOP, Pos2::new(10.0, 10.0))
            .direction(Direction::TopDown);


        let worker_receiver = self.worker.receiver();
        while let Ok(response) = worker_receiver.try_recv() {
            match response {
                WorkerResponse::OpenedSourceImage { image, file_path } => {
                    let image_texture = allocate_texture_for_rgba8_image(
                        &image,
                        &ctx.tex_manager(),
                    );

                    let image_aspect_ratio =
                        image.width() as f32 / image.height() as f32;

                    self.state.source_image = Some(SourceImage {
                        file_path,
                        image: Arc::new(image),
                        image_aspect_ratio,
                        image_texture,
                    });

                    self.state.is_loading_image = false;
                }
                WorkerResponse::FailedToOpenSourceImage { error } => {
                    let error_text = match error {
                        ImageLoadError::FileReadError { error } => {
                            format!(
                                "Failed to read input file.\n\nContext: {error}"
                            )
                        }
                        ImageLoadError::ImageParseError { error } => {
                            format!(
                                "Failed to parse input file. Maybe not in a valid format?\n\nContext: {error}"
                            )
                        }
                    };

                    toasts.add(
                        egui_toast::Toast::default()
                            .text(error_text)
                            .kind(egui_toast::ToastKind::Error)
                            .options(
                                egui_toast::ToastOptions::default()
                                    .duration(None)
                                    .show_progress(false)
                                    .show_icon(true),
                            ),
                    );

                    self.state.is_loading_image = false;
                }
                WorkerResponse::ProcessedImage { image } => {
                    let image_texture = allocate_texture_for_rgba8_image(
                        &image,
                        &ctx.tex_manager(),
                    );

                    let image_aspect_ratio =
                        image.width() as f32 / image.height() as f32;

                    self.state.processed_image = Some(ProcessedImage {
                        image: Arc::new(image),
                        image_aspect_ratio,
                        image_texture,
                    });

                    self.state.is_processing_image = false;
                }
                WorkerResponse::SavedImage { output_file_path } => {
                    toasts.add(
                        egui_toast::Toast::default()
                            .text(format!("Image successfully saved to disk.\n\nFull path: {}", output_file_path.to_string_lossy()))
                            .kind(egui_toast::ToastKind::Success)
                            .options(
                                egui_toast::ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true)
                                    .show_icon(true),
                            ),
                    );

                    self.state.is_saving_image = false;
                }
                WorkerResponse::FailedToSaveImage { error } => {
                    let error_text = match error {
                        ImageSaveError::FileOpenError { error } => {
                            format!(
                                "Failed to open output file.\n\nContext: {error}"
                            )
                        }
                        ImageSaveError::ImageError { error } => {
                            format!(
                                "Failed to encode or write image.\n\nContext: {error}"
                            )
                        }
                        ImageSaveError::FileFlushError { error } => {
                            format!(
                                "Failed to flush and/or close the file.\n\nContext: {error}"
                            )
                        }
                    };

                    toasts.add(
                        egui_toast::Toast::default()
                            .text(error_text)
                            .kind(egui_toast::ToastKind::Error)
                            .options(
                                egui_toast::ToastOptions::default()
                                    .duration(None)
                                    .show_progress(false)
                                    .show_icon(true),
                            ),
                    );

                    self.state.is_saving_image = false;
                }
            }
        }


        CentralPanel::default().show(ctx, |ui| {
            egui_taffy::tui(ui, ui.id().with("root"))
                .reserve_available_space()
                .style(taffy::Style {
                    display: taffy::Display::Flex,
                    flex_direction: taffy::FlexDirection::Row,
                    justify_items: Some(taffy::JustifyItems::Stretch),
                    align_items: Some(taffy::AlignItems::Stretch),
                    size: taffy::Size::from_percent(1.0, 1.0),
                    ..Default::default()
                })
                .show(|taffy_ui| {
                    self.central_view.update(taffy_ui, &mut self.state);

                    taffy_ui.separator();

                    self.right_sidebar.update(
                        taffy_ui,
                        ctx,
                        frame,
                        &self.worker,
                        &mut self.state,
                    );
                });
        });

        toasts.show(ctx);
    }
}
