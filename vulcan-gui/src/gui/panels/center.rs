use egui_taffy::{Tui, TuiBuilderLogic, taffy};

use crate::{gui::SharedState, utilities::select_first_some_3};

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
                let image_context = select_first_some_3(
                    state.threshold_preview.as_ref().map(|preview| {
                        (preview.image_texture, preview.image_aspect_ratio)
                    }),
                    state.processed_image_last.as_ref().map(|last| {
                        (last.image_texture, last.image_aspect_ratio)
                    }),
                    state.source_image.as_ref().map(|source| {
                        (source.image_texture, source.image_aspect_ratio)
                    }),
                );

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
