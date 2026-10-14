use freya::prelude::*;

use crate::{state::ModManagerContext, widgets::AppearanceMode};

use super::{
    table::{BulkActions, ModTable},
    toolbar::ModToolbar,
    widgets::AppearanceSettings,
};

#[derive(PartialEq)]
pub struct App;

impl Component for App {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let appearance = *context.appearance.read();

        let root = rect().expanded().padding(16.0).spacing(12.0);

        let root = match appearance {
            AppearanceMode::Acrylic => root,
            AppearanceMode::Transparent => root.background(Color::TRANSPARENT),
            AppearanceMode::Color => root.theme_background(),
        };

        root.child(
            rect()
                .theme_color()
                .expanded()
                .width(Size::fill())
                .corner_radius(8.0)
                .border(Border::new().fill(get_theme_or_default().read().colors.border).width(1.0))
                .spacing(0.0)
                .child(AppearanceSettings)
                .child(ModToolbar)
                .child(rect().expanded().width(Size::fill()).child(ModTable))
                .child(BulkActions),
        )
    }
}
