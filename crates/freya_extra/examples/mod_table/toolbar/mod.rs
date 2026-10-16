mod column_filter;
mod density_select;
mod quick_search;

pub use column_filter::ColumnFilterEditor as ColumnFilter;
pub use density_select::DensitySelect;
use freya::prelude::*;
pub use quick_search::QuickSearch;

use crate::state::ModManagerContext;

#[derive(PartialEq)]
pub struct ModToolbar;

impl Component for ModToolbar {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let mut enabled_only = context.enabled_only;

        let enabled_only_switch =
            rect().horizontal().cross_align(Alignment::Center).spacing(6.0).child(
                Tile::new()
                    .on_select(move |_| enabled_only.toggle())
                    .child(Checkbox::new().selected(!enabled_only()))
                    .child("Enabled only"),
            );

        // Intended:
        // ┌─────────────────--------─────────────────────────────────────┐
        // │ Mods                    [Search] [Filter] [Switch] [Density] │
        // └────────────────────────--------──────────────────────────────┘
        rect()
            .horizontal()
            .main_align(Alignment::SpaceBetween)
            .spacing(8.0)
            .width(Size::fill())
            .padding(Gaps::new(8.0, 12.0, 8.0, 12.0))
            .child(label().text("Mods").font_size(20.0).font_weight(FontWeight::BOLD))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .child(QuickSearch)
                    .child(ColumnFilter)
                    .child(enabled_only_switch)
                    .child(DensitySelect),
            )
    }
}
