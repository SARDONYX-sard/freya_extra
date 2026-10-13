use freya::prelude::*;

use super::hex;

pub fn theme() -> Theme {
    let mut theme = dark_theme();

    theme.name = "liquid-glass";
    theme.colors = LIQUID_GLASS_COLORS;
    theme
}

pub const LIQUID_GLASS_COLORS: ColorsSheet = ColorsSheet {
    // Brand and Accent
    primary: hex("#B8B8B8FF"),
    secondary: hex("#D9D9D9FF"),
    tertiary: hex("#EEEEEEFF"),

    // Status
    success: hex("#75C9A0FF"),
    warning: hex("#E8C778FF"),
    error: hex("#E98282FF"),
    info: hex("#B9C7D6FF"),

    // Translucent Glass Surfaces
    background: hex("#77777720"),
    surface_primary: hex("#a7a4a49a"),
    surface_secondary: hex("#64646411"),
    surface_tertiary: hex("#ffffff31"),
    surface_inverse: hex("#292929E8"),
    surface_inverse_secondary: hex("#383838D8"),
    surface_inverse_tertiary: hex("#4A4A4AC8"),

    // Glass Edges
    border: hex("#FFFFFF55"),
    border_focus: hex("#FFFFFFFF"),
    border_disabled: hex("#FFFFFF24"),

    // Text
    text_primary: hex("#F7F7F7FF"),
    text_secondary: hex("#DDDDDDFF"),
    text_placeholder: hex("#B5B5B5CC"),
    text_inverse: hex("#292929FF"),
    text_highlight: hex("#FFFFFFFF"),

    // Interaction
    focus: hex("#D0D0D0FF"),
    active: hex("#FFFFFF40"),
    disabled: hex("#B0B0B066"),

    // Utility
    overlay: hex("#10101048"),
    shadow: hex("#00000030"),
};
