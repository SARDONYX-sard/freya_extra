use freya::prelude::*;
use freya_extra::{acrylic, dropdown_menu::DropdownMenu, themes};

use crate::state::ModManagerContext;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppearanceMode {
    #[default]
    Acrylic,
    Transparent,
    Color,
}

#[derive(PartialEq)]
pub struct AppearanceSettings;

impl Component for AppearanceSettings {
    fn render(&self) -> impl IntoElement {
        let mut appearance = use_consume::<ModManagerContext>().appearance;
        let mut error_message = use_state(String::new);
        let err_msg = error_message.read().clone();

        let appearance_select = DropdownMenu::new(appearance())
            .item(AppearanceMode::Acrylic, "Acrylic")
            .item(AppearanceMode::Transparent, "Transparent")
            .item(AppearanceMode::Color, "Color")
            .on_change(move |next| {
                let should_enable_acrylic = next == AppearanceMode::Acrylic;
                let is_enabled = acrylic::is_enabled();

                if should_enable_acrylic == is_enabled {
                    appearance.set(next);
                    error_message.set(String::new());
                    return;
                }

                match acrylic::toggle() {
                    Ok(_) => {
                        error_message.set(String::new());
                        appearance.set(next);
                    }
                    Err(error) => {
                        error_message.set(error.to_string());
                    }
                }
            });

        rect()
            .horizontal()
            .spacing(16.0)
            .cross_align(Alignment::Center)
            .child(themes::editor::ThemeEditor)
            .child(
                rect()
                    .horizontal()
                    .spacing(8.0)
                    .cross_align(Alignment::Center)
                    .child(label().text("Appearance"))
                    .child(appearance_select)
                    .child(label().color(Color::RED).text(err_msg)),
            )
    }
}
