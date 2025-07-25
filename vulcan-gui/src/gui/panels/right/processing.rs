use egui::Color32;
use egui_taffy::{Tui, TuiBuilderLogic, taffy};
use vulcan_core::sorting::{
    ImageSortingDirection,
    PixelSegmentSelectionMode,
    PixelSegmentSortDirection,
    PixelSortOptions,
};

use crate::{
    gui::{SharedState, panels::ConditionalDisabledTuiBuilder},
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
                ImageSortingDirection::Horizontal(
                    PixelSegmentSortDirection::Ascending,
                )
            }
            UiImageSortingDirection::HorizontalDescending => {
                ImageSortingDirection::Horizontal(
                    PixelSegmentSortDirection::Descending,
                )
            }
            UiImageSortingDirection::VerticalAscending => {
                ImageSortingDirection::Vertical(
                    PixelSegmentSortDirection::Ascending,
                )
            }
            UiImageSortingDirection::VerticalDescending => {
                ImageSortingDirection::Vertical(
                    PixelSegmentSortDirection::Descending,
                )
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPixelSegmentSelectionMode {
    LuminanceRange,
    HueRange,
    SaturationRange,
    CannyEdges,
}

impl UiPixelSegmentSelectionMode {
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
            UiPixelSegmentSelectionMode::LuminanceRange => "relative luminance range",
            UiPixelSegmentSelectionMode::HueRange => "hue range",
            UiPixelSegmentSelectionMode::SaturationRange => "saturation range",
            UiPixelSegmentSelectionMode::CannyEdges => "edge-to-edge (canny)",
        }
    }
}

pub struct UiPixelSegmentSelectionState {
    mode: UiPixelSegmentSelectionMode,

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
            mode: UiPixelSegmentSelectionMode::LuminanceRange,
            luminance_range_low: 0.0,
            luminance_range_high: 1.0,
            hue_range_low: 0.0,
            hue_range_high: 360.0,
            saturation_range_low: 0.0,
            saturation_range_high: 1.0,
            canny_edges_low: 0.0,
            canny_edges_high: 1.0,
            canny_edges_segment_starts_on_image_edge: false,
        }
    }

    pub fn selection_mode(&self) -> PixelSegmentSelectionMode {
        match self.mode {
            UiPixelSegmentSelectionMode::LuminanceRange => {
                PixelSegmentSelectionMode::LuminanceRange {
                    low: self.luminance_range_low,
                    high: self.luminance_range_high,
                }
            }
            UiPixelSegmentSelectionMode::HueRange => {
                PixelSegmentSelectionMode::HueRange {
                    low: self.hue_range_low,
                    high: self.hue_range_high,
                }
            }
            UiPixelSegmentSelectionMode::SaturationRange => {
                PixelSegmentSelectionMode::SaturationRange {
                    low: self.saturation_range_low,
                    high: self.saturation_range_high,
                }
            }
            UiPixelSegmentSelectionMode::CannyEdges => {
                PixelSegmentSelectionMode::CannyEdges {
                    low: self.canny_edges_low,
                    high: self.canny_edges_high,
                    segment_starts_on_image_edge: self
                        .canny_edges_segment_starts_on_image_edge,
                }
            }
        }
    }
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

pub struct ImageProcessingSection {
    segment_selection_state: UiPixelSegmentSelectionState,
    segment_sorting_direction: UiImageSortingDirection,
}

impl ImageProcessingSection {
    pub fn new() -> Self {
        Self {
            segment_selection_state: UiPixelSegmentSelectionState::new(),
            segment_sorting_direction:
                UiImageSortingDirection::HorizontalAscending,
        }
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
                            .selected_text(
                                self.segment_selection_state.mode.label(),
                            )
                            .show_ui(ui, |ui| {
                                for mode in UiPixelSegmentSelectionMode::modes()
                                {
                                    ui.selectable_value(
                                        &mut self.segment_selection_state.mode,
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

                match self.segment_selection_state.mode {
                    UiPixelSegmentSelectionMode::LuminanceRange => {
                        taffy_ui
                            .style(segment_selection_mode_dropdown_style.clone())
                            .ui(|ui| {
                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .luminance_range_low,
                                    )
                                    .text("Low threshold"),
                                );

                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .luminance_range_high,
                                    )
                                    .text("High threshold"),
                                );
                            });
                    }
                    UiPixelSegmentSelectionMode::HueRange => {
                        taffy_ui
                            .style(segment_selection_mode_dropdown_style.clone())
                            .ui(|ui| {
                                ui.add(
                                    construct_precise_hue_slider(
                                        &mut self
                                            .segment_selection_state
                                            .hue_range_low,
                                    )
                                    .text("Low threshold"),
                                );

                                ui.add(
                                    construct_precise_hue_slider(
                                        &mut self
                                            .segment_selection_state
                                            .hue_range_high,
                                    )
                                    .text("High threshold"),
                                );
                            });
                    },
                    UiPixelSegmentSelectionMode::SaturationRange => {
                        taffy_ui
                            .style(segment_selection_mode_dropdown_style.clone())
                            .ui(|ui| {
                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .saturation_range_low,
                                    )
                                    .text("Low threshold"),
                                );

                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .saturation_range_high,
                                    )
                                    .text("High threshold"),
                                );
                            });
                    },
                    UiPixelSegmentSelectionMode::CannyEdges => {
                        taffy_ui
                            .style(segment_selection_mode_dropdown_style.clone())
                            .ui(|ui| {
                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .canny_edges_low,
                                    )
                                    .text("Low threshold"),
                                );

                                ui.add(
                                    construct_precise_normalized_slider(
                                        &mut self
                                            .segment_selection_state
                                            .canny_edges_high,
                                    )
                                    .text("High threshold"),
                                );

                                ui.add(egui::Checkbox::new(
                                    &mut self
                                        .segment_selection_state
                                        .canny_edges_segment_starts_on_image_edge,
                                    "First segment starts on edge"
                                ));
                            });
                    },
                }

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
                            .selected_text(
                                self.segment_sorting_direction.label(),
                            )
                            .show_ui(ui, |ui| {
                                for direction in
                                    UiImageSortingDirection::directions()
                                {
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
                        let reset_button = taffy_ui
                            .style(taffy::Style {
                                flex_grow: 4.0,
                                min_size: taffy::Size {
                                    width: taffy::Dimension::Length(50.0),
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
                            .disabled_if(state.processed_image.is_none())
                            .ui_add(egui::Button::new(egui_phosphor::regular::BACKSPACE).fill(Color32::TRANSPARENT))
                            .on_hover_text("Reset view to source image.")
                            .on_disabled_hover_text("Cannot reset to source image: no processed image yet.");

                        if reset_button.clicked() {
                            if let Some(processed_image) = state.processed_image.take() {
                                let texture_manager = ctx.tex_manager();
                                let mut locked_texture_manager = texture_manager.write();

                                locked_texture_manager.free(processed_image.image_texture.id);

                                drop(processed_image);
                            }
                        }


                        let sorting_button = taffy_ui
                            .style(taffy::Style {
                                flex_grow: 4.0,
                                min_size: taffy::Size {
                                    width: taffy::Dimension::Percent(0.75),
                                    height: taffy::Dimension::Length(14.0),
                                },
                                max_size: taffy::Size {
                                    width: taffy::Dimension::Percent(1.0),
                                    height: taffy::Dimension::Length(20.0),
                                },
                                ..Default::default()
                            })
                            .ui_add(egui::Button::new("Execute pixel sort"))
                            .on_hover_text(
                                "Performs pixel sorting, always using the source image. \
                                If you want apply sorting to a processed image instead, manually export and re-import the image."
                            );

                        if sorting_button.clicked()
                            && let Some(source_image) = &state.source_image
                        {
                            let _ = worker.sender().send(
                                WorkerRequest::PerformPixelSorting {
                                    image: source_image.image.clone(),
                                    method: self
                                        .segment_selection_state
                                        .selection_mode(),
                                    options: PixelSortOptions {
                                        direction: self
                                            .segment_sorting_direction
                                            .to_image_sorting_direction(),
                                    },
                                },
                            );

                            state.is_processing_image = true;
                        }

                        if state.is_processing_image {
                            taffy_ui
                                .style(taffy::Style {
                                    flex_grow: 1.0,
                                    margin: taffy::Rect {
                                        left: taffy::LengthPercentageAuto::Length(0.0),
                                        bottom: taffy::LengthPercentageAuto::Length(4.0),
                                        right: taffy::LengthPercentageAuto::Length(0.0),
                                        top: taffy::LengthPercentageAuto::Length(2.0),
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
                                        left: taffy::LengthPercentageAuto::Length(0.0),
                                        bottom: taffy::LengthPercentageAuto::Length(4.0),
                                        right: taffy::LengthPercentageAuto::Length(0.0),
                                        top: taffy::LengthPercentageAuto::Length(2.0),
                                    },
                                    size: taffy::Size {
                                        width: taffy::Dimension::Length(spinner_style),
                                        height: taffy::Dimension::Length(spinner_style)
                                    },
                                    ..Default::default()
                                })
                                .add_empty();
                        }
                    });
            });
    }
}
