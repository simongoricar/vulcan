use std::{ops::RangeInclusive, time::Instant};

use egui::Color32;
use egui_taffy::{Tui, TuiBuilderLogic, taffy};
use vulcan_core::{
    feedback::FeedbackSegmentSelectionMode,
    pixel_sorting::{
        ImageSortingDirection,
        PixelSegmentSortDirection,
        prepared::{PreparedSegmentSelectionMode, PreparedSegmentSortingMode},
    },
};

use crate::{
    gui::{
        ProcessedImage,
        SharedState,
        allocate_texture_for_rgba8_image,
        free_texture,
        panels::ConditionalDisabledTuiBuilder,
    },
    utilities::select_first_some,
    worker::{WorkerHandle, WorkerRequest},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiImageSortingDirection {
    HorizontalAscending,
    HorizontalDescending,
    VerticalAscending,
    VerticalDescending,
}

impl UiImageSortingDirection {
    pub fn directions() -> [Self; 4] {
        [
            Self::HorizontalAscending,
            Self::HorizontalDescending,
            Self::VerticalAscending,
            Self::VerticalDescending,
        ]
    }

    #[rustfmt::skip]
    pub fn label(self) -> &'static str {
        match self {
            UiImageSortingDirection::HorizontalAscending => "horizontal, ascending",
            UiImageSortingDirection::HorizontalDescending => "horizontal, descending",
            UiImageSortingDirection::VerticalAscending => "vertical, ascending",
            UiImageSortingDirection::VerticalDescending => "vertical, descending"
        }
    }

    pub fn to_image_sorting_direction(self) -> ImageSortingDirection {
        match self {
            UiImageSortingDirection::HorizontalAscending => {
                ImageSortingDirection::Horizontal(PixelSegmentSortDirection::Ascending)
            }
            UiImageSortingDirection::HorizontalDescending => {
                ImageSortingDirection::Horizontal(PixelSegmentSortDirection::Descending)
            }
            UiImageSortingDirection::VerticalAscending => {
                ImageSortingDirection::Vertical(PixelSegmentSortDirection::Ascending)
            }
            UiImageSortingDirection::VerticalDescending => {
                ImageSortingDirection::Vertical(PixelSegmentSortDirection::Descending)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum UiSegmentSelectionMode {
    LuminanceRange,
    HueRange,
    SaturationRange,
    CannyEdges,
}

impl UiSegmentSelectionMode {
    pub fn modes() -> [Self; 4] {
        [
            Self::LuminanceRange,
            Self::HueRange,
            Self::SaturationRange,
            Self::CannyEdges,
        ]
    }

    #[rustfmt::skip]
    pub fn label(self) -> &'static str {
        match self {
            UiSegmentSelectionMode::LuminanceRange => "relative luminance range",
            UiSegmentSelectionMode::HueRange => "hue range",
            UiSegmentSelectionMode::SaturationRange => "saturation range",
            UiSegmentSelectionMode::CannyEdges => "edge-to-edge (canny)",
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiSortingMode {
    Luminance,
    Hue,
    Saturation,
}

impl UiSortingMode {
    pub fn modes() -> [Self; 3] {
        [Self::Luminance, Self::Hue, Self::Saturation]
    }

    #[rustfmt::skip]
    pub fn label(self) -> &'static str {
        match self {
            Self::Luminance => "luminance",
            Self::Hue => "hue",
            Self::Saturation => "saturation",
        }
    }

    pub fn to_prepared_sorting_mode(self) -> PreparedSegmentSortingMode {
        match self {
            Self::Luminance => PreparedSegmentSortingMode::Luminance,
            Self::Hue => PreparedSegmentSortingMode::Hue,
            Self::Saturation => PreparedSegmentSortingMode::Saturation,
        }
    }
}


const SMALLEST_CANNY_EDGE_THRESHOLD: f32 = 0.1;

/// See <https://docs.rs/imageproc/latest/imageproc/edges/fn.canny.html> for more information.
const LARGEST_CANNY_EDGE_THRESHOLD: f32 = 1140.39;


pub struct UiPixelSegmentSelectionState {
    segment_selection_mode: UiSegmentSelectionMode,
    sorting_mode: UiSortingMode,

    luminance_range_low: f32,
    luminance_range_high: f32,
    hue_range_low: f32,
    hue_range_high: f32,
    saturation_range_low: f32,
    saturation_range_high: f32,
    canny_edges_low: f32,
    canny_edges_high: f32,
    canny_edges_segment_starts_on_image_edge: bool,
}

impl UiPixelSegmentSelectionState {
    pub fn new() -> Self {
        Self {
            segment_selection_mode: UiSegmentSelectionMode::LuminanceRange,
            sorting_mode: UiSortingMode::Luminance,
            luminance_range_low: 0.0,
            luminance_range_high: 1.0,
            hue_range_low: 0.0,
            hue_range_high: 360.0,
            saturation_range_low: 0.0,
            saturation_range_high: 1.0,
            canny_edges_low: SMALLEST_CANNY_EDGE_THRESHOLD,
            canny_edges_high: LARGEST_CANNY_EDGE_THRESHOLD,
            canny_edges_segment_starts_on_image_edge: false,
        }
    }

    // pub fn selection_mode(&self) -> ImmediateSegmentSelectionMode {
    //     match self.mode {
    //         UiImmediateSegmentSelectionMode::LuminanceRange => {
    //             ImmediateSegmentSelectionMode::LuminanceRange {
    //                 low: self.luminance_range_low,
    //                 high: self.luminance_range_high,
    //             }
    //         }
    //         UiImmediateSegmentSelectionMode::HueRange => ImmediateSegmentSelectionMode::HueRange {
    //             low: self.hue_range_low,
    //             high: self.hue_range_high,
    //         },
    //         UiImmediateSegmentSelectionMode::SaturationRange => {
    //             ImmediateSegmentSelectionMode::SaturationRange {
    //                 low: self.saturation_range_low,
    //                 high: self.saturation_range_high,
    //             }
    //         }
    //     }
    // }
}

fn construct_precise_normalized_slider(value: &mut f32) -> egui::Slider {
    egui::Slider::new(value, 0.0..=1.0)
        .step_by(0.0001)
        .min_decimals(4)
        .max_decimals(6)
        .drag_value_speed(0.0001)
}

fn construct_precise_hue_slider(value: &mut f32) -> egui::Slider {
    egui::Slider::new(value, 0.0..=360.0)
        .step_by(0.001)
        .min_decimals(4)
        .max_decimals(6)
        .drag_value_speed(0.001)
}


fn construct_precise_custom_slider(value: &mut f32, range: RangeInclusive<f32>) -> egui::Slider {
    egui::Slider::new(value, range)
        .step_by(0.0001)
        .min_decimals(4)
        .max_decimals(6)
        .drag_value_speed(0.0001)
}


pub struct ImageProcessingSection {
    segment_selection_state: UiPixelSegmentSelectionState,
    segment_sorting_direction: UiImageSortingDirection,
}

impl ImageProcessingSection {
    pub fn new() -> Self {
        Self {
            segment_selection_state: UiPixelSegmentSelectionState::new(),
            segment_sorting_direction: UiImageSortingDirection::HorizontalAscending,
        }
    }

    fn handle_threshold_preview_state(
        &mut self,
        should_display_preview: bool,
        feedback_mode: FeedbackSegmentSelectionMode,
        worker: &WorkerHandle,
        ctx: &egui::Context,
        state: &mut SharedState,
    ) {
        if !state.is_waiting_for_updated_preview && should_display_preview {
            let image_to_preview_on = select_first_some(
                state.processed_image_last.as_ref().map(|last| &last.image),
                state.source_image.as_ref().map(|source| &source.image),
            );

            let last_preview_time = state
                .threshold_preview
                .as_ref()
                .map(|preview| preview.last_redraw);

            let should_redraw_preview = last_preview_time
                .map(|time| time.elapsed().as_secs_f32() >= 0.1)
                .unwrap_or(true);

            if should_redraw_preview && let Some(image_to_preview_on) = image_to_preview_on {
                let _ = worker.sender().send(WorkerRequest::ShowThresholdPreview {
                    image: image_to_preview_on.clone(),
                    method: feedback_mode,
                    requested_at: Instant::now(),
                });

                state.is_waiting_for_updated_preview = true;
            }

            let time_since_last_hover_instant = state.last_threshold_hover_time.elapsed();
            if time_since_last_hover_instant.as_secs_f32() > 0.1 {
                state.last_threshold_hover_time = Instant::now();
            }
        } else {
            let time_since_hover_left = state.last_threshold_hover_time.elapsed();

            if time_since_hover_left.as_secs_f32() > 0.5
                && let Some(existing_preview) = state.threshold_preview.take()
            {
                free_texture(
                    &ctx.tex_manager(),
                    existing_preview.image_texture.id,
                );
            }
        }
    }

    fn update_sorting_ui_actions(
        &mut self,
        taffy_ui: &mut Tui,
        worker: &WorkerHandle,
        ctx: &egui::Context,
        state: &mut SharedState,
    ) {
        let reset_button = taffy_ui
            .style(taffy::Style {
                flex_grow: 4.0,
                min_size: taffy::Size {
                    width: taffy::Dimension::Length(20.0),
                    height: taffy::Dimension::Length(24.0),
                },
                max_size: taffy::Size {
                    width: taffy::Dimension::Auto,
                    height: taffy::Dimension::Length(32.0),
                },
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(8.0),
                    top: taffy::LengthPercentageAuto::Length(0.0),
                    bottom: taffy::LengthPercentageAuto::Length(0.0),
                },
                ..Default::default()
            })
            .disabled_if(state.processed_image_last.is_none())
            .ui_add(egui::Button::new(egui_phosphor::regular::BACKSPACE).fill(Color32::TRANSPARENT))
            .on_hover_text("Reset view to source image.")
            .on_disabled_hover_text("Cannot reset to source image: no processed image yet.");

        if reset_button.clicked() {
            if let Some(processed_image) = state.processed_image_last.take() {
                let texture_manager = ctx.tex_manager();
                let mut locked_texture_manager = texture_manager.write();

                locked_texture_manager.free(processed_image.image_texture.id);

                drop(processed_image);
            }
        }

        let undo_button = taffy_ui
            .style(taffy::Style {
                flex_grow: 4.0,
                min_size: taffy::Size {
                    width: taffy::Dimension::Length(20.0),
                    height: taffy::Dimension::Length(24.0),
                },
                max_size: taffy::Size {
                    width: taffy::Dimension::Auto,
                    height: taffy::Dimension::Length(32.0),
                },
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(14.0),
                    top: taffy::LengthPercentageAuto::Length(0.0),
                    bottom: taffy::LengthPercentageAuto::Length(0.0),
                },
                ..Default::default()
            })
            .disabled_if(
                state.processed_image_history_stack.is_empty()
                    || state.processed_image_last.is_none(),
            )
            .ui_add(
                egui::Button::new(egui_phosphor::regular::CLOCK_COUNTER_CLOCKWISE)
                    .fill(Color32::TRANSPARENT),
            )
            .on_hover_text(format!(
                "Undo by one step ({} history entries available).",
                state.processed_image_history_stack.len()
            ))
            .on_disabled_hover_text("Undo unavailable: no entries in history stack.");

        if undo_button.clicked() {
            let last_history_entry = state.processed_image_history_stack.pop();

            if let Some(last_history_entry) = last_history_entry {
                let current_last_procesed = state
                    .processed_image_last
                    .take()
                    .expect("non-empty history stack, but no last processed image?!");

                let texture_manager = ctx.tex_manager();
                {
                    let mut locked_texture_manager = texture_manager.write();
                    locked_texture_manager.free(current_last_procesed.image_texture.id);
                };

                let allocated_texture =
                    allocate_texture_for_rgba8_image(&last_history_entry.image, &texture_manager);

                state.processed_image_last = Some(ProcessedImage {
                    image: last_history_entry.image,
                    image_aspect_ratio: last_history_entry.image_aspect_ratio,
                    image_texture: allocated_texture,
                });
            }
        }

        let sorting_button = taffy_ui
            .style(taffy::Style {
                flex_grow: 4.0,
                min_size: taffy::Size {
                    width: taffy::Dimension::Length(150.0),
                    height: taffy::Dimension::Length(24.0),
                },
                max_size: taffy::Size {
                    width: taffy::Dimension::Auto,
                    height: taffy::Dimension::Length(32.0),
                },
                ..Default::default()
            })
            .ui_add(egui::Button::new("Execute pixel sort"));
        // .on_hover_text(
        //     "Performs pixel sorting, always using the source image. \
        //     If you want apply sorting to a processed image instead, manually export and re-import the image."
        // );

        #[allow(clippy::manual_map)]
        if sorting_button.clicked() {
            let image_to_sort = if let Some(processed_image_state) = &state.processed_image_last {
                Some(processed_image_state.image.clone())
            } else if let Some(source_image_state) = &state.source_image {
                Some(source_image_state.image.clone())
            } else {
                None
            };

            if let Some(image_to_sort) = image_to_sort {
                let sorting_mode = self
                    .segment_selection_state
                    .sorting_mode
                    .to_prepared_sorting_mode();

                let sorting_direction = self.segment_sorting_direction.to_image_sorting_direction();

                let message_to_send: WorkerRequest = match self
                    .segment_selection_state
                    .segment_selection_mode
                {
                    UiSegmentSelectionMode::LuminanceRange => {
                        WorkerRequest::PerformPreparedPixelSorting {
                            image: image_to_sort,
                            segment_selection_mode: PreparedSegmentSelectionMode::LuminanceRange {
                                low: self.segment_selection_state.luminance_range_low,
                                high: self.segment_selection_state.luminance_range_high,
                            },
                            sorting_mode,
                            sorting_direction,
                        }
                    }
                    UiSegmentSelectionMode::HueRange => {
                        WorkerRequest::PerformPreparedPixelSorting {
                            image: image_to_sort,
                            segment_selection_mode: PreparedSegmentSelectionMode::HueRange {
                                low: self.segment_selection_state.hue_range_low,
                                high: self.segment_selection_state.hue_range_high,
                            },
                            sorting_mode,
                            sorting_direction,
                        }
                    }
                    UiSegmentSelectionMode::SaturationRange => {
                        WorkerRequest::PerformPreparedPixelSorting {
                            image: image_to_sort,
                            segment_selection_mode: PreparedSegmentSelectionMode::SaturationRange {
                                low: self.segment_selection_state.saturation_range_low,
                                high: self.segment_selection_state.saturation_range_high,
                            },
                            sorting_mode,
                            sorting_direction,
                        }
                    }
                    UiSegmentSelectionMode::CannyEdges => {
                        WorkerRequest::PerformPreparedPixelSorting {
                            image: image_to_sort,
                            segment_selection_mode: PreparedSegmentSelectionMode::CannyEdges {
                                low: self.segment_selection_state.canny_edges_low,
                                high: self.segment_selection_state.canny_edges_high,
                                segment_starts_on_image_edge: self
                                    .segment_selection_state
                                    .canny_edges_segment_starts_on_image_edge,
                            },
                            sorting_mode,
                            sorting_direction,
                        }
                    }
                };

                let _ = worker.sender().send(message_to_send);

                state.is_processing_image = true;
            }
        }

        if state.is_processing_image {
            taffy_ui
                .style(taffy::Style {
                    flex_grow: 1.0,
                    margin: taffy::Rect {
                        left: taffy::LengthPercentageAuto::Length(10.0),
                        bottom: taffy::LengthPercentageAuto::Length(0.0),
                        right: taffy::LengthPercentageAuto::Length(0.0),
                        top: taffy::LengthPercentageAuto::Length(0.0),
                    },
                    ..Default::default()
                })
                .ui_add(egui::Spinner::new());
        } else {
            let spinner_style = taffy_ui.egui_ui_mut().style().spacing.interact_size.y;

            taffy_ui
                .style(taffy::Style {
                    flex_grow: 1.0,
                    margin: taffy::Rect {
                        left: taffy::LengthPercentageAuto::Length(10.0),
                        bottom: taffy::LengthPercentageAuto::Length(0.0),
                        right: taffy::LengthPercentageAuto::Length(0.0),
                        top: taffy::LengthPercentageAuto::Length(0.0),
                    },
                    size: taffy::Size {
                        width: taffy::Dimension::Length(spinner_style),
                        height: taffy::Dimension::Length(spinner_style),
                    },
                    ..Default::default()
                })
                .add_empty();
        }
    }

    fn update_sorting_ui(
        &mut self,
        taffy_ui: &mut Tui,
        worker: &WorkerHandle,
        ctx: &egui::Context,
        state: &mut SharedState,
    ) {
        taffy_ui
            .style(taffy::Style {
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(4.0),
                    bottom: taffy::LengthPercentageAuto::Length(14.0),
                },
                ..Default::default()
            })
            .ui(|ui| {
                egui::ComboBox::from_label("Segment selection mode")
                    .selected_text(self.segment_selection_state.segment_selection_mode.label())
                    .show_ui(ui, |ui| {
                        for mode in UiSegmentSelectionMode::modes() {
                            ui.selectable_value(
                                &mut self.segment_selection_state.segment_selection_mode,
                                mode,
                                mode.label(),
                            );
                        }
                    })
            });

        let segment_selection_mode_dropdown_style = taffy::Style {
            display: taffy::Display::Flex,
            flex_direction: taffy::FlexDirection::Column,
            align_items: Some(taffy::AlignItems::Center),
            margin: taffy::Rect {
                left: taffy::LengthPercentageAuto::Length(0.0),
                right: taffy::LengthPercentageAuto::Length(0.0),
                top: taffy::LengthPercentageAuto::Length(4.0),
                bottom: taffy::LengthPercentageAuto::Length(2.0),
            },
            ..Default::default()
        };

        match self.segment_selection_state.segment_selection_mode {
            UiSegmentSelectionMode::LuminanceRange => {
                taffy_ui
                    .style(segment_selection_mode_dropdown_style.clone())
                    .ui(|ui| {
                        let low_threshold = ui.add(
                            construct_precise_normalized_slider(
                                &mut self.segment_selection_state.luminance_range_low,
                            )
                            .text("Low threshold"),
                        );

                        let high_threshold = ui.add(
                            construct_precise_normalized_slider(
                                &mut self.segment_selection_state.luminance_range_high,
                            )
                            .text("High threshold"),
                        );

                        let should_display_preview = low_threshold.contains_pointer()
                            || low_threshold.dragged()
                            || low_threshold.changed()
                            || high_threshold.contains_pointer()
                            || high_threshold.dragged()
                            || high_threshold.changed();

                        self.handle_threshold_preview_state(
                            should_display_preview,
                            FeedbackSegmentSelectionMode::LuminanceRange {
                                low: self.segment_selection_state.luminance_range_low,
                                high: self.segment_selection_state.luminance_range_high,
                            },
                            worker,
                            ctx,
                            state,
                        );
                    });
            }
            UiSegmentSelectionMode::HueRange => {
                taffy_ui
                    .style(segment_selection_mode_dropdown_style.clone())
                    .ui(|ui| {
                        let low_hue_threshold = ui.add(
                            construct_precise_hue_slider(
                                &mut self.segment_selection_state.hue_range_low,
                            )
                            .text("Low threshold"),
                        );

                        let high_hue_threshold = ui.add(
                            construct_precise_hue_slider(
                                &mut self.segment_selection_state.hue_range_high,
                            )
                            .text("High threshold"),
                        );

                        let should_display_preview = low_hue_threshold.contains_pointer()
                            || low_hue_threshold.dragged()
                            || low_hue_threshold.changed()
                            || high_hue_threshold.contains_pointer()
                            || high_hue_threshold.dragged()
                            || high_hue_threshold.changed();

                        self.handle_threshold_preview_state(
                            should_display_preview,
                            FeedbackSegmentSelectionMode::HueRange {
                                low: self.segment_selection_state.hue_range_low,
                                high: self.segment_selection_state.hue_range_high,
                            },
                            worker,
                            ctx,
                            state,
                        );
                    });
            }
            UiSegmentSelectionMode::SaturationRange => {
                taffy_ui
                    .style(segment_selection_mode_dropdown_style.clone())
                    .ui(|ui| {
                        let saturation_threshold_low = ui.add(
                            construct_precise_normalized_slider(
                                &mut self.segment_selection_state.saturation_range_low,
                            )
                            .text("Low threshold"),
                        );

                        let saturation_threshold_high = ui.add(
                            construct_precise_normalized_slider(
                                &mut self.segment_selection_state.saturation_range_high,
                            )
                            .text("High threshold"),
                        );

                        let should_display_preview = saturation_threshold_low.contains_pointer()
                            || saturation_threshold_low.dragged()
                            || saturation_threshold_low.changed()
                            || saturation_threshold_high.contains_pointer()
                            || saturation_threshold_high.dragged()
                            || saturation_threshold_high.changed();

                        self.handle_threshold_preview_state(
                            should_display_preview,
                            FeedbackSegmentSelectionMode::SaturationRange {
                                low: self.segment_selection_state.saturation_range_low,
                                high: self.segment_selection_state.saturation_range_high,
                            },
                            worker,
                            ctx,
                            state,
                        );
                    });
            }
            UiSegmentSelectionMode::CannyEdges => {
                taffy_ui
                    .style(segment_selection_mode_dropdown_style.clone())
                    .ui(|ui| {
                        ui.add(
                            construct_precise_custom_slider(
                                &mut self.segment_selection_state.canny_edges_low,
                                SMALLEST_CANNY_EDGE_THRESHOLD..=LARGEST_CANNY_EDGE_THRESHOLD,
                            )
                            .text("Low edge threshold"),
                        );

                        ui.add(
                            construct_precise_custom_slider(
                                &mut self.segment_selection_state.canny_edges_high,
                                SMALLEST_CANNY_EDGE_THRESHOLD..=LARGEST_CANNY_EDGE_THRESHOLD,
                            )
                            .text("High edge threshold"),
                        );

                        ui.add(egui::Checkbox::new(
                            &mut self
                                .segment_selection_state
                                .canny_edges_segment_starts_on_image_edge,
                            "First segment starts on left/top of image",
                        ));
                    });
            }
        }

        taffy_ui
            .style(taffy::Style {
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(14.0),
                    bottom: taffy::LengthPercentageAuto::Length(8.0),
                },
                ..Default::default()
            })
            .ui(|ui| -> egui::InnerResponse<Option<()>> {
                egui::ComboBox::from_label("Segment sorting mode")
                    .selected_text(self.segment_selection_state.sorting_mode.label())
                    .show_ui(ui, |ui| {
                        for mode in UiSortingMode::modes() {
                            ui.selectable_value(
                                &mut self.segment_selection_state.sorting_mode,
                                mode,
                                mode.label(),
                            );
                        }
                    })
            });

        taffy_ui
            .style(taffy::Style {
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(8.0),
                    bottom: taffy::LengthPercentageAuto::Length(12.0),
                },
                ..Default::default()
            })
            .ui(|ui| {
                egui::ComboBox::from_label("Segment sorting direction")
                    .selected_text(self.segment_sorting_direction.label())
                    .show_ui(ui, |ui| {
                        for direction in UiImageSortingDirection::directions() {
                            ui.selectable_value(
                                &mut self.segment_sorting_direction,
                                direction,
                                direction.label(),
                            );
                        }
                    })
            });

        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Row,
                justify_items: Some(taffy::JustifyItems::Center),
                align_items: Some(taffy::AlignItems::Center),
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(12.0),
                    bottom: taffy::LengthPercentageAuto::Length(2.0),
                },
                ..Default::default()
            })
            .add(|taffy_ui| {
                self.update_sorting_ui_actions(taffy_ui, worker, ctx, state);
            });
    }

    pub(super) fn update(
        &mut self,
        taffy_ui: &mut Tui,
        worker: &WorkerHandle,
        ctx: &egui::Context,
        state: &mut SharedState,
    ) {
        taffy_ui
            .style(taffy::Style {
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(10.0),
                    bottom: taffy::LengthPercentageAuto::Length(10.0),
                },
                ..Default::default()
            })
            .separator();

        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                justify_content: Some(taffy::JustifyContent::Start),
                justify_items: Some(taffy::JustifyItems::Start),
                align_content: Some(taffy::AlignContent::Start),
                align_items: Some(taffy::AlignItems::Center),
                margin: taffy::Rect::length(8.0),
                min_size: taffy::Size {
                    width: taffy::Dimension::Percent(1.0),
                    height: taffy::Dimension::Auto,
                },
                ..Default::default()
            })
            .add(|taffy_ui| {
                taffy_ui
                    .style(taffy::Style {
                        min_size: taffy::Size {
                            width: taffy::Dimension::Percent(1.0),
                            height: taffy::Dimension::Auto,
                        },
                        margin: taffy::Rect {
                            left: taffy::LengthPercentageAuto::Length(0.0),
                            right: taffy::LengthPercentageAuto::Length(0.0),
                            top: taffy::LengthPercentageAuto::Length(6.0),
                            bottom: taffy::LengthPercentageAuto::Length(12.0),
                        },
                        ..Default::default()
                    })
                    .ui(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(format!(
                                "{} Processing",
                                egui_phosphor::regular::SWAP,
                            ))
                            .size(16.0),
                        ))
                    });

                self.update_sorting_ui(taffy_ui, worker, ctx, state);
            });
    }
}
