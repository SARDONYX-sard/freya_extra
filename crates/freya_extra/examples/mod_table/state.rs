use freya::prelude::*;
use freya_extra::dnd_table::Density;

use crate::{
    model::{ColumnFilter, ModRow, initial_rows},
    widgets::AppearanceMode,
};

#[derive(Clone, Copy)]
pub struct ModManagerContext {
    pub rows: State<Vec<ModRow>>,
    pub search: State<String>,
    pub filters: State<Vec<ColumnFilter>>,
    pub enabled_only: State<bool>,
    pub density: State<Density>,
    pub menu: State<Option<(f64, f64)>>,
    pub menu_hover: State<Option<usize>>,
    pub appearance: State<AppearanceMode>,
}

impl ModManagerContext {
    pub fn new() -> Self {
        Self {
            rows: State::create(initial_rows()),
            search: State::create(String::new()),
            filters: State::create(Vec::new()),
            enabled_only: State::create(false),
            density: State::create(Density::default()),
            menu: State::create(None),
            menu_hover: State::create(None),
            appearance: State::create(AppearanceMode::default()),
        }
    }
}

impl Default for ModManagerContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_state() {
    use_provide_context(ModManagerContext::new);
}
