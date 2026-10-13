use freya::prelude::*;

use super::hex;

pub fn theme() -> Theme {
    let mut theme = dark_theme();

    theme.name = "dark-glass";
    theme.colors = DARK_GLASS_COLORS;

    theme
}

pub const DARK_GLASS_COLORS: ColorsSheet = ColorsSheet {
    // Brand & Accent
    primary: hex("#4d6277"),
    secondary: hex("#64B5FFFF"),
    tertiary: hex("#4A63A0FF"),

    // Status / Semantic colors
    success: hex("#30D158FF"),
    warning: hex("#FF9F0AFF"),
    error: hex("#FF453AFF"),
    info: hex("#64D2FFFF"),

    // Background and Glass Surfaces
    background: hex("#0c101862"),
    surface_primary: hex("#232b37b2"),
    surface_secondary: hex("#525d6da1"),
    surface_tertiary: hex("#414B5A9B"),
    surface_inverse: hex("#F5F7FAF5"),
    surface_inverse_secondary: hex("#E0E5EDE6"),
    surface_inverse_tertiary: hex("#C6CDD8D2"),

    // Borders / Glass Edges
    border: hex("#EBF2FF64"),
    border_focus: hex("#64B5FFD2"),
    border_disabled: hex("#C8D2E137"),

    // Text / Content
    text_primary: hex("#FAFAFCFF"),
    text_secondary: hex("#CDD3DEE1"),
    text_placeholder: hex("#AAB4C3AF"),
    text_inverse: hex("#1C1D21FF"),
    text_highlight: hex("#82C3FFFF"),

    // States / Interaction
    focus: hex("#0A84FFFF"),
    active: hex("#3791F5D2"),
    disabled: hex("#9199A66E"),

    // Utility
    overlay: hex("#00000096"),
    shadow: hex("#00000064"),
};
