use std::{
    path::PathBuf,
    sync::Arc,
    time::{Instant, UNIX_EPOCH},
};

use eframe::App;
use egui::{
    Align2,
    CentralPanel,
    ColorImage,
    Direction,
    ImageData,
    Pos2,
    TextureId,
    TextureOptions,
    Vec2,
    epaint::{ImageDelta, TextureManager},
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

pub struct ThresholdPreview {
    image_aspect_ratio: f32,
    image_texture: SizedTexture,
    last_redraw: Instant,
}

pub struct ProcessedImageHistoryEntry {
    image: Arc<RgbaImage>,
    image_aspect_ratio: f32,
}

pub struct SharedState {
    source_image: Option<SourceImage>,

    /// Represents the history stack of the processing. Separated from the last image,
    /// as the history stack doesn't have an allocated texture.
    processed_image_history_stack: Vec<ProcessedImageHistoryEntry>,

    /// The last (current) state of the processed image. In contrast to `processed_image_history_stack`, this
    processed_image_last: Option<ProcessedImage>,

    threshold_preview: Option<ThresholdPreview>,
    is_waiting_for_updated_preview: bool,

    last_threshold_hover_time: Instant,

    is_loading_image: bool,
    is_processing_image: bool,
    is_saving_image: bool,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            source_image: None,
            processed_image_history_stack: Vec::new(),
            processed_image_last: None,
            threshold_preview: None,
            last_threshold_hover_time: Instant::now(),
            is_waiting_for_updated_preview: false,
            is_loading_image: false,
            is_processing_image: false,
            is_saving_image: false,
        }
    }
}

pub(crate) fn update_full_texture_using_rgba8_image(
    image: &RgbaImage,
    texture_manager: &RwLock<TextureManager>,
    existing_texture_id: TextureId,
) {
    let epaint_color_image = ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_flat_samples().as_slice(),
    );

    let epaint_image_data = ImageData::Color(Arc::new(epaint_color_image));

    let mut locked_texture_manager = texture_manager.write();

    locked_texture_manager.set(
        existing_texture_id,
        ImageDelta::full(epaint_image_data, TextureOptions::LINEAR),
    );
}

pub(crate) fn allocate_texture_for_rgba8_image(
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

pub(crate) fn free_texture(texture_manager: &RwLock<TextureManager>, texture_id: TextureId) {
    let mut locked_texture_manager = texture_manager.write();

    locked_texture_manager.free(texture_id);
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
        let mut toasts = egui_toast::Toasts::new()
            .anchor(Align2::LEFT_TOP, Pos2::new(10.0, 10.0))
            .direction(Direction::TopDown);

        let worker_receiver = self.worker.receiver();
        while let Ok(response) = worker_receiver.try_recv() {
            match response {
                WorkerResponse::OpenedSourceImage { image, file_path } => {
                    if let Some(previous_source_image) = self.state.source_image.take() {
                        let texture_manager = ctx.tex_manager();
                        let mut locked_texture_manager = texture_manager.write();

                        locked_texture_manager.free(previous_source_image.image_texture.id);
                    }

                    let image_texture =
                        allocate_texture_for_rgba8_image(&image, &ctx.tex_manager());

                    let image_aspect_ratio = image.width() as f32 / image.height() as f32;

                    self.state.source_image = Some(SourceImage {
                        file_path,
                        image: Arc::new(image),
                        image_aspect_ratio,
                        image_texture,
                    });

                    self.state.processed_image_history_stack.clear();

                    if let Some(previous_processed_image) = self.state.processed_image_last.take() {
                        let texture_manager = ctx.tex_manager();
                        let mut locked_texture_manager = texture_manager.write();

                        locked_texture_manager.free(previous_processed_image.image_texture.id);
                    }

                    self.state.is_loading_image = false;
                }
                WorkerResponse::FailedToOpenSourceImage { error } => {
                    let error_text = match error {
                        ImageLoadError::FileReadError { error } => {
                            format!("Failed to read input file.\n\nContext: {error}")
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
                    if let Some(previous_processed_image) = self.state.processed_image_last.take() {
                        let texture_manager = ctx.tex_manager();
                        let mut locked_texture_manager = texture_manager.write();

                        locked_texture_manager.free(previous_processed_image.image_texture.id);

                        self.state
                            .processed_image_history_stack
                            .push(ProcessedImageHistoryEntry {
                                image: previous_processed_image.image,
                                image_aspect_ratio: previous_processed_image.image_aspect_ratio,
                            });
                    }

                    let image_texture =
                        allocate_texture_for_rgba8_image(&image, &ctx.tex_manager());

                    let image_aspect_ratio = image.width() as f32 / image.height() as f32;

                    self.state.processed_image_last = Some(ProcessedImage {
                        image: Arc::new(image),
                        image_aspect_ratio,
                        image_texture,
                    });

                    self.state.is_processing_image = false;
                }
                WorkerResponse::ProcessedThresholdPreview {
                    image,
                    requested_at,
                } => {
                    if self.state.is_waiting_for_updated_preview {
                        self.state.is_waiting_for_updated_preview = false;
                        let texture_manager = ctx.tex_manager();

                        if let Some(previous_preview) = self.state.threshold_preview.as_mut() {
                            update_full_texture_using_rgba8_image(
                                &image,
                                &texture_manager,
                                previous_preview.image_texture.id,
                            );

                            previous_preview.image_aspect_ratio =
                                (image.width() as f32) / (image.height() as f32);
                            previous_preview.last_redraw = requested_at;
                        } else {
                            let texture =
                                allocate_texture_for_rgba8_image(&image, &texture_manager);

                            self.state.threshold_preview = Some(ThresholdPreview {
                                image_aspect_ratio: (image.width() as f32)
                                    / (image.height() as f32),
                                image_texture: texture,
                                last_redraw: requested_at,
                            });
                        }
                    }
                }
                WorkerResponse::SavedImage { output_file_path } => {
                    toasts.add(
                        egui_toast::Toast::default()
                            .text(format!(
                                "Image successfully saved to disk.\n\nFull path: {}",
                                output_file_path.to_string_lossy()
                            ))
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
                            format!("Failed to open output file.\n\nContext: {error}")
                        }
                        ImageSaveError::ImageError { error } => {
                            format!("Failed to encode or write image.\n\nContext: {error}")
                        }
                        ImageSaveError::FileFlushError { error } => {
                            format!("Failed to flush and/or close the file.\n\nContext: {error}")
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
