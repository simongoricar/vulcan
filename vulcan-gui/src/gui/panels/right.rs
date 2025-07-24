use egui::{AtomExt, Color32, Frame, Ui};
use egui_taffy::{
    Tui,
    TuiBuilderLogic,
    taffy::{
        self,
        AlignItems,
        AlignSelf,
        Display,
        FlexDirection,
        JustifyItems,
        JustifySelf,
        LengthPercentage,
        Style,
        prelude::{auto, length, percent},
    },
    tui,
};

use crate::worker::{WorkerHandle, WorkerRequest};

pub struct RightSidebar {
    image_load_section: ImageLoadSection,
    image_processing_section: ImageProcessingSection,
    image_save_section: ImageSaveSection,
}

impl RightSidebar {
    pub fn new() -> Self {
        Self {
            image_load_section: ImageLoadSection::new(),
            image_processing_section: ImageProcessingSection::new(),
            image_save_section: ImageSaveSection::new(),
        }
    }

    pub fn update(
        &mut self,
        taffy_ui: &mut Tui,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        worker: &WorkerHandle,
    ) {
        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                justify_content: Some(taffy::JustifyContent::Stretch),
                justify_items: Some(taffy::JustifyItems::Stretch),
                align_content: Some(taffy::AlignContent::Stretch),
                align_items: Some(taffy::AlignItems::Stretch),
                flex_grow: 1.0,
                min_size: taffy::Size {
                    height: taffy::Dimension::Percent(1.0),
                    width: taffy::Dimension::Percent(0.35),
                },
                ..Default::default()
            })
            .add(|taffy_ui| {
                self.image_load_section.update(taffy_ui, worker);
                self.image_processing_section
                    .update(taffy_ui, ctx, frame, worker);
            });
    }
}

pub struct ImageLoadSection {}

impl ImageLoadSection {
    pub fn new() -> Self {
        Self {}
    }

    pub(super) fn update(&mut self, taffy_ui: &mut Tui, worker: &WorkerHandle) {
        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                justify_content: Some(taffy::JustifyContent::Start),
                justify_items: Some(taffy::JustifyItems::Start),
                align_content: Some(taffy::AlignContent::Start),
                align_items: Some(taffy::AlignItems::Center),
                margin: length(3.0),
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
                            top: taffy::LengthPercentageAuto::Length(2.0),
                            bottom: taffy::LengthPercentageAuto::Length(6.0),
                        },
                        ..Default::default()
                    })
                    .ui(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(format!(
                                "{} Input file",
                                egui_phosphor::regular::UPLOAD_SIMPLE,
                            ))
                            .size(16.0),
                        ))
                    });

                let file_picker_button = taffy_ui
                    .style(taffy::Style {
                        min_size: taffy::Size {
                            width: taffy::Dimension::Percent(0.75),
                            height: taffy::Dimension::Auto,
                        },
                        max_size: taffy::Size {
                            width: taffy::Dimension::Percent(1.0),
                            height: taffy::Dimension::Length(20.0),
                        },
                        ..Default::default()
                    })
                    .ui_add(egui::Button::new(
                        egui::RichText::new(format!(
                            "{} Open file",
                            egui_phosphor::regular::FOLDER_OPEN
                        ))
                        .size(14f32),
                    ));

                if file_picker_button.clicked() {
                    let optionally_picked_file = rfd::FileDialog::new()
                        .set_title("Input file")
                        .add_filter("Image", &["png", "jpg", "jpeg"])
                        .pick_file();

                    if let Some(picked_file) = optionally_picked_file {
                        let _ = worker.sender().send(
                            WorkerRequest::OpenSourceFile {
                                file_path: picked_file,
                            },
                        );
                    }
                }
            });

        // TODO
        // todo!();
    }
}


pub struct ImageProcessingSection {
    threshold_low: f32,
    threshold_high: f32,
}

impl ImageProcessingSection {
    pub fn new() -> Self {
        Self {
            threshold_low: 0.0,
            threshold_high: 1.0,
        }
    }

    pub(super) fn update(
        &mut self,
        taffy_ui: &mut Tui,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        worker: &WorkerHandle,
    ) {
        taffy_ui
            .style(taffy::Style {
                margin: taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(0.0),
                    right: taffy::LengthPercentageAuto::Length(0.0),
                    top: taffy::LengthPercentageAuto::Length(8.0),
                    bottom: taffy::LengthPercentageAuto::Length(8.0),
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
                margin: length(3.0),
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
                            top: taffy::LengthPercentageAuto::Length(2.0),
                            bottom: taffy::LengthPercentageAuto::Length(6.0),
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
                        display: taffy::Display::Flex,
                        flex_direction: taffy::FlexDirection::Column,
                        align_items: Some(taffy::AlignItems::Center),
                        ..Default::default()
                    })
                    .ui(|ui| {
                        ui.add(
                            egui::Slider::new(
                                &mut self.threshold_low,
                                0.0..=1.0,
                            )
                            .step_by(0.0001)
                            .min_decimals(4)
                            .max_decimals(5)
                            .drag_value_speed(0.001)
                            .text("Low threshold"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.threshold_high,
                                0.0..=1.0,
                            )
                            .step_by(0.0001)
                            .min_decimals(4)
                            .max_decimals(5)
                            .drag_value_speed(0.001)
                            .text("High threshold"),
                        );
                    });
            });

        // todo!();
    }
}



pub struct ImageSaveSection {}

impl ImageSaveSection {
    pub fn new() -> Self {
        Self {}
    }

    pub(super) fn update(
        &mut self,
        tui: &mut Tui,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        todo!();
    }
}
