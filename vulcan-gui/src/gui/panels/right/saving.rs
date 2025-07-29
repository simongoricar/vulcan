use egui_taffy::{Tui, TuiBuilderLogic, taffy};

use crate::{
    gui::SharedState,
    worker::{WorkerHandle, WorkerRequest},
};

pub struct ImageSaveSection {}

impl ImageSaveSection {
    pub fn new() -> Self {
        Self {}
    }

    pub(super) fn update(
        &mut self,
        taffy_ui: &mut Tui,
        state: &mut SharedState,
        worker: &WorkerHandle,
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
                            top: taffy::LengthPercentageAuto::Length(2.0),
                            bottom: taffy::LengthPercentageAuto::Length(16.0),
                        },
                        ..Default::default()
                    })
                    .ui(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(format!(
                                "{} Save file",
                                egui_phosphor::regular::EXPORT,
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
                            "{} Save file",
                            egui_phosphor::regular::EXPORT
                        ))
                        .size(14f32),
                    ));

                if file_picker_button.clicked() {
                    #[allow(clippy::manual_map)]
                    let image_to_save =
                        if let Some(processed_image_state) = &state.processed_image_last {
                            Some(processed_image_state.image.clone())
                        } else if let Some(source_image_state) = &state.source_image {
                            Some(source_image_state.image.clone())
                        } else {
                            None
                        };

                    if let Some(image_to_save) = image_to_save {
                        let starting_file_name = state
                            .source_image
                            .as_ref()
                            .and_then(|source| {
                                source
                                    .file_path
                                    .with_extension("png")
                                    .file_name()
                                    .map(|name| name.to_string_lossy().to_string())
                            })
                            .unwrap_or("sorted-image.png".to_string());

                        let optional_output_file_path = rfd::FileDialog::new()
                            .set_title("Save file")
                            .set_file_name(starting_file_name)
                            .save_file();

                        if let Some(mut output_file_path) = optional_output_file_path {
                            if output_file_path.extension().is_none() {
                                output_file_path.set_extension("png");
                            }

                            let _ = worker.sender().send(WorkerRequest::SaveImage {
                                image: image_to_save,
                                output_file_path,
                            });

                            state.is_saving_image = true;
                        }
                    }
                }

                if state.is_saving_image {
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
