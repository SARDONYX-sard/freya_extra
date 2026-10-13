use freya::prelude::*;

use super::liquid_glass::LIQUID_GLASS_COLORS;
use crate::color_picker::ColorPicker;
use crate::dropdown_menu::DropdownMenu;
use crate::themes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeMode {
    Dark,
    DarkGlass,
    Light,
    LiquidGlass,
    Custom,
}

#[derive(Clone, Copy)]
enum ColorField {
    Primary,
    Secondary,
    Tertiary,
    Success,
    Warning,
    Error,
    Info,
    Background,
    SurfacePrimary,
    SurfaceSecondary,
    SurfaceTertiary,
    SurfaceInverse,
    SurfaceInverseSecondary,
    SurfaceInverseTertiary,
    Border,
    BorderFocus,
    BorderDisabled,
    TextPrimary,
    TextSecondary,
    TextPlaceholder,
    TextInverse,
    TextHighlight,
    Focus,
    Active,
    Disabled,
    Overlay,
    Shadow,
}

impl ColorField {
    const ALL: [Self; 27] = [
        Self::Primary,
        Self::Secondary,
        Self::Tertiary,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Info,
        Self::Background,
        Self::SurfacePrimary,
        Self::SurfaceSecondary,
        Self::SurfaceTertiary,
        Self::SurfaceInverse,
        Self::SurfaceInverseSecondary,
        Self::SurfaceInverseTertiary,
        Self::Border,
        Self::BorderFocus,
        Self::BorderDisabled,
        Self::TextPrimary,
        Self::TextSecondary,
        Self::TextPlaceholder,
        Self::TextInverse,
        Self::TextHighlight,
        Self::Focus,
        Self::Active,
        Self::Disabled,
        Self::Overlay,
        Self::Shadow,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
            Self::Tertiary => "Tertiary",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Info => "Info",
            Self::Background => "Background",
            Self::SurfacePrimary => "Surface Primary",
            Self::SurfaceSecondary => "Surface Secondary",
            Self::SurfaceTertiary => "Surface Tertiary",
            Self::SurfaceInverse => "Surface Inverse",
            Self::SurfaceInverseSecondary => "Surface Inverse Secondary",
            Self::SurfaceInverseTertiary => "Surface Inverse Tertiary",
            Self::Border => "Border",
            Self::BorderFocus => "Border Focus",
            Self::BorderDisabled => "Border Disabled",
            Self::TextPrimary => "Text Primary",
            Self::TextSecondary => "Text Secondary",
            Self::TextPlaceholder => "Text Placeholder",
            Self::TextInverse => "Text Inverse",
            Self::TextHighlight => "Text Highlight",
            Self::Focus => "Focus",
            Self::Active => "Active",
            Self::Disabled => "Disabled",
            Self::Overlay => "Overlay",
            Self::Shadow => "Shadow",
        }
    }

    fn get(self, colors: &ColorsSheet) -> Color {
        match self {
            Self::Primary => colors.primary,
            Self::Secondary => colors.secondary,
            Self::Tertiary => colors.tertiary,
            Self::Success => colors.success,
            Self::Warning => colors.warning,
            Self::Error => colors.error,
            Self::Info => colors.info,
            Self::Background => colors.background,
            Self::SurfacePrimary => colors.surface_primary,
            Self::SurfaceSecondary => colors.surface_secondary,
            Self::SurfaceTertiary => colors.surface_tertiary,
            Self::SurfaceInverse => colors.surface_inverse,
            Self::SurfaceInverseSecondary => colors.surface_inverse_secondary,
            Self::SurfaceInverseTertiary => colors.surface_inverse_tertiary,
            Self::Border => colors.border,
            Self::BorderFocus => colors.border_focus,
            Self::BorderDisabled => colors.border_disabled,
            Self::TextPrimary => colors.text_primary,
            Self::TextSecondary => colors.text_secondary,
            Self::TextPlaceholder => colors.text_placeholder,
            Self::TextInverse => colors.text_inverse,
            Self::TextHighlight => colors.text_highlight,
            Self::Focus => colors.focus,
            Self::Active => colors.active,
            Self::Disabled => colors.disabled,
            Self::Overlay => colors.overlay,
            Self::Shadow => colors.shadow,
        }
    }

    fn set(self, colors: &mut ColorsSheet, color: Color) {
        match self {
            Self::Primary => colors.primary = color,
            Self::Secondary => colors.secondary = color,
            Self::Tertiary => colors.tertiary = color,
            Self::Success => colors.success = color,
            Self::Warning => colors.warning = color,
            Self::Error => colors.error = color,
            Self::Info => colors.info = color,
            Self::Background => colors.background = color,
            Self::SurfacePrimary => colors.surface_primary = color,
            Self::SurfaceSecondary => colors.surface_secondary = color,
            Self::SurfaceTertiary => colors.surface_tertiary = color,
            Self::SurfaceInverse => colors.surface_inverse = color,
            Self::SurfaceInverseSecondary => colors.surface_inverse_secondary = color,
            Self::SurfaceInverseTertiary => colors.surface_inverse_tertiary = color,
            Self::Border => colors.border = color,
            Self::BorderFocus => colors.border_focus = color,
            Self::BorderDisabled => colors.border_disabled = color,
            Self::TextPrimary => colors.text_primary = color,
            Self::TextSecondary => colors.text_secondary = color,
            Self::TextPlaceholder => colors.text_placeholder = color,
            Self::TextInverse => colors.text_inverse = color,
            Self::TextHighlight => colors.text_highlight = color,
            Self::Focus => colors.focus = color,
            Self::Active => colors.active = color,
            Self::Disabled => colors.disabled = color,
            Self::Overlay => colors.overlay = color,
            Self::Shadow => colors.shadow = color,
        }
    }
}

fn custom_theme(colors: ColorsSheet) -> Theme {
    let mut theme = dark_theme();
    theme.name = "custom";
    theme.colors = colors;
    theme
}

#[derive(PartialEq)]
pub struct ThemeEditor;

impl Component for ThemeEditor {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let current_name = theme.read().name;

        let mode = match current_name {
            value if value == light_theme().name => ThemeMode::Light,
            value if value == dark_theme().name => ThemeMode::Dark,
            value if value == themes::liquid_glass::theme().name => ThemeMode::LiquidGlass,
            value if value == themes::dark_glass::theme().name => ThemeMode::DarkGlass,
            _ => ThemeMode::Custom,
        };

        let custom_colors = use_state(|| LIQUID_GLASS_COLORS);
        let mut show_editor = use_state(|| false);

        let mut theme_for_dropdown = theme;
        let colors_for_dropdown = custom_colors;

        let theme_dropdown = DropdownMenu::new(mode)
            .item(ThemeMode::Dark, "Dark")
            .item(ThemeMode::Light, "Light")
            .item(ThemeMode::LiquidGlass, "LiquidGlass")
            .item(ThemeMode::DarkGlass, "DarkGlass")
            .item(ThemeMode::Custom, "Custom")
            .on_change(move |selected| match selected {
                ThemeMode::Dark => theme_for_dropdown.set(dark_theme()),
                ThemeMode::Light => theme_for_dropdown.set(light_theme()),
                ThemeMode::LiquidGlass => theme_for_dropdown.set(themes::liquid_glass::theme()),
                ThemeMode::DarkGlass => theme_for_dropdown.set(themes::dark_glass::theme()),
                ThemeMode::Custom => {
                    theme_for_dropdown.set(custom_theme(colors_for_dropdown.read().clone()));
                }
            });

        let mut selector =
            rect().horizontal().spacing(8.0).cross_align(Alignment::Center).child(theme_dropdown);

        if mode == ThemeMode::Custom {
            selector = selector
                .child(Button::new().on_press(move |_| show_editor.set(true)).child("Edit"));
        }

        let colors = custom_colors;
        let active_theme = theme;

        let floating_window = DraggableCanvas::new().maybe_child(if show_editor() {
            Some(
                Draggable::new().initial_position((40.0, 40.0)).child(
                    rect()
                        .layer(999)
                        .width(Size::px(420.0))
                        .height(Size::px(560.0))
                        .padding(12.0)
                        .spacing(8.0)
                        .vertical()
                        .child(
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(8.0)
                                .child(label().text("Custom Theme").font_size(16.0))
                                .child(
                                    Button::new()
                                        .on_press(move |_| show_editor.set(false))
                                        .child("Close"),
                                ),
                        )
                        .child(
                            VirtualScrollView::new(move |item, _| {
                                let field = ColorField::ALL[item.index];

                                let snapshot = colors.read().clone();
                                let current_color = field.get(&snapshot);

                                let mut row_colors = colors;
                                let mut row_theme = active_theme;

                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .width(Size::fill())
                                    .height(Size::px(item.size))
                                    .padding(6.0)
                                    .spacing(8.0)
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .child(label().text(field.name()).font_size(13.0)),
                                    )
                                    .child(
                                        ColorPicker::new(move |color| {
                                            let mut updated = row_colors.read().clone();

                                            field.set(&mut updated, color);

                                            row_colors.set(updated.clone());
                                            row_theme.set(custom_theme(updated));
                                        })
                                        .value(current_color),
                                    )
                                    .into()
                            })
                            .length(ColorField::ALL.len())
                            .item_size(64.0)
                            .width(Size::fill())
                            .max_height(Size::px(470.0)),
                        ),
                ),
            )
        } else {
            None
        });

        rect()
            .horizontal()
            .spacing(8.0)
            .cross_align(Alignment::Center)
            .child(selector)
            .child(floating_window)
    }
}
