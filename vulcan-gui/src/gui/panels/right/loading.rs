use egui_taffy::{Tui, TuiBuilderLogic, taffy};

use crate::{
    gui::SharedState,
    worker::{WorkerHandle, WorkerRequest},
};

pub struct ImageLoadSection {}

impl ImageLoadSection {
    pub fn new() -> Self {
        Self {}
    }

    pub(super) fn update(
        &mut self,
        taffy_ui: &mut Tui,
        worker: &WorkerHandle,
        state: &mut SharedState,
    ) {
        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                justify_content: Some(taffy::JustifyContent::Start),
                justify_items: Some(taffy::JustifyItems::Start),
                align_content: Some(taffy::AlignContent::Start),
                align_items: Some(taffy::AlignItems::Center),
                margin: taffy::Rect::length(10.0),
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
                            WorkerRequest::OpenSourceImage {
                                input_file_path: picked_file,
                            },
                        );

                        state.is_loading_image = true;
                    }
                }

                if state.is_loading_image {
                    taffy_ui
                        .style(taffy::Style {
                            margin: taffy::Rect {
                                left: taffy::LengthPercentageAuto::Length(0.0),
                                bottom: taffy::LengthPercentageAuto::Length(4.0),
                                right: taffy::LengthPercentageAuto::Length(0.0),
                                top: taffy::LengthPercentageAuto::Length(2.0),
                            },
                            ..Default::default()
                        })
                        .ui(|ui| ui.add(egui::Spinner::new()));
                }
            });
    }
}
