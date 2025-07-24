use egui_taffy::{AsTuiBuilder, TuiBuilder, TuiBuilderLogic};

pub mod center;
pub mod right;

pub trait ConditionalDisabledTuiBuilder<'r>: TuiBuilderLogic<'r> {
    fn disabled_if(self, is_disabled: bool) -> TuiBuilder<'r> {
        if is_disabled {
            self.disabled()
        } else {
            self.tui()
        }
    }
}

impl<'r, T> ConditionalDisabledTuiBuilder<'r> for T where T: AsTuiBuilder<'r> {}
