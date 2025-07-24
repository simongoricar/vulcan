use egui::Vec2;
use egui_taffy::{Tui, TuiBuilderLogic, taffy};

use crate::{gui::SharedState, worker::WorkerHandle};

pub struct CentralView {}

impl CentralView {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(clippy::manual_map)]
    pub fn update(&mut self, taffy_ui: &mut Tui, state: &mut SharedState) {
        taffy_ui
            .style(taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                justify_content: Some(taffy::JustifyContent::Start),
                justify_items: Some(taffy::JustifyItems::Start),
                align_content: Some(taffy::AlignContent::Start),
                align_items: Some(taffy::AlignItems::Start),
                min_size: taffy::Size {
                    height: taffy::Dimension::Percent(1.0),
                    width: taffy::Dimension::Percent(0.65),
                },
                ..Default::default()
            })
            .add(|taffy_ui| {
                let image_context =
                    if let Some(processed_image) = &state.processed_image {
                        Some((
                            processed_image.image_texture,
                            processed_image.image_aspect_ratio,
                        ))
                    } else if let Some(source_image) = &state.source_image {
                        Some((
                            source_image.image_texture,
                            source_image.image_aspect_ratio,
                        ))
                    } else {
                        None
                    };

                if let Some((sized_texture, aspect_ratio)) = image_context {
                    taffy_ui
                        .style(taffy::Style {
                            flex_grow: 1.0,
                            flex_basis: taffy::Dimension::Percent(1.0),
                            size: taffy::Size {
                                width: taffy::Dimension::Percent(1.0),
                                height: taffy::Dimension::Auto,
                            },
                            aspect_ratio: Some(aspect_ratio),
                            ..Default::default()
                        })
                        .ui(|ui| {
                            let available_size = ui.available_size();

                            println!(
                                "image available_size = {available_size:?}"
                            );

                            let image_widget =
                                egui::Image::from_texture(sized_texture)
                                    .max_size(available_size);

                            ui.add_sized(available_size, image_widget)
                        });
                } else {
                    taffy_ui.add_empty();
                }

                // todo!();
            });
    }
}
