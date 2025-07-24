use egui_taffy::{Tui, TuiBuilderLogic, taffy};

use crate::{
    gui::{
        SharedState,
        panels::right::{
            loading::ImageLoadSection,
            processing::ImageProcessingSection,
            saving::ImageSaveSection,
        },
    },
    worker::WorkerHandle,
};

mod loading;
mod processing;
mod saving;


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
        _ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        worker: &WorkerHandle,
        state: &mut SharedState,
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
                self.image_load_section.update(taffy_ui, worker, state);
                self.image_processing_section
                    .update(taffy_ui, worker, state);
                self.image_save_section.update(taffy_ui, state, worker);
            });
    }
}
