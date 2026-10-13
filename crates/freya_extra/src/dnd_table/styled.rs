// Decorated parts: colors, opacity, animation. Structural containers live in the components.

use std::time::Duration;

use freya::{animation::Function, prelude::*};

use super::{TableMetrics, calc::HandleState};

// Every color comes from the active theme.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct TableColors {
    pub header_background: Color,
    pub header_button_background: Color,
    pub header_text: Color,
    pub row_even: Color,
    pub row_odd: Color,
    pub grid_line: Color,
    pub insert_line: Color,
    pub resize_hover: Color,
    pub resize_active: Color,
    pub ghost_background: Color,
}

pub(super) fn use_table_colors() -> TableColors {
    let colors = use_theme().read().colors.clone();

    TableColors {
        header_background: colors.surface_secondary,
        header_button_background: colors.surface_tertiary,
        header_text: colors.text_primary,
        row_even: colors.surface_primary,
        row_odd: Color::lerp(colors.surface_primary, colors.surface_secondary, 0.35),
        grid_line: Color::lerp(colors.border, colors.background, 0.65),
        insert_line: colors.primary,
        resize_hover: colors.border_focus,
        resize_active: colors.primary,
        ghost_background: colors.surface_tertiary,
    }
}

pub(super) fn row_background(
    is_header: bool,
    position: Option<usize>,
    colors: TableColors,
) -> Color {
    if is_header {
        colors.header_background
    } else if position.unwrap_or(0).is_multiple_of(2) {
        colors.row_even
    } else {
        colors.row_odd
    }
}

pub(super) fn header_label(text: String, colors: TableColors) -> Element {
    label()
        .text(text)
        .color(colors.header_text)
        .font_weight(FontWeight::BOLD)
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis)
        .into()
}

// Transparent unless it is the current drop boundary.
pub(super) fn insert_line(height: f32, active: bool, colors: TableColors) -> Element {
    rect()
        .width(Size::fill())
        .height(Size::px(height))
        .spacing(0.0)
        .background(if active { colors.insert_line } else { Color::TRANSPARENT })
        .into()
}

pub(super) fn grid_separator(colors: TableColors) -> Element {
    rect().width(Size::px(1.0)).height(Size::fill()).background(colors.grid_line).into()
}

pub(super) fn resize_line(
    state: HandleState,
    metrics: TableMetrics,
    colors: TableColors,
) -> Element {
    let (width, color) = match state {
        HandleState::Active => (metrics.resize_line_hover_width, colors.resize_active),
        HandleState::Hovered => (metrics.resize_line_hover_width, colors.resize_hover),
        HandleState::Idle => (metrics.resize_line_width, colors.grid_line),
    };

    rect().width(Size::px(width)).height(Size::fill()).background(color).into()
}

pub(super) fn row_opacity(is_dragged: bool) -> f32 {
    if is_dragged { 0.5 } else { 1.0 }
}

pub(super) fn row_portal(id: usize, width: Size, height: f32) -> Portal<usize> {
    Portal::new(id)
        .width(width)
        .height(Size::px(height))
        .function(Function::Expo)
        .duration(Duration::from_millis(200))
}

// Must not intercept pointer events.
pub(super) fn drag_ghost(
    width: Size,
    height: f32,
    colors: TableColors,
    content: impl Into<Element>,
) -> Rect {
    rect()
        .interactive(false)
        .background(colors.ghost_background)
        .layer(999)
        .width(width)
        .height(Size::px(height))
        .opacity(0.75)
        .child(content)
}
