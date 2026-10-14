use freya::prelude::*;
use freya_extra::{dnd_table::Density, dropdown_menu::DropdownMenu};

use crate::state::ModManagerContext;

#[derive(PartialEq)]
pub struct DensitySelect;

impl Component for DensitySelect {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let density = *context.density.read();

        DropdownMenu::new(density)
            .item(Density::Compact, "Compact")
            .item(Density::Standard, "Standard")
            .item(Density::Comfortable, "Comfortable")
            .on_change(move |next| {
                let mut state = context.density;
                state.set(next);
            })
    }
}
