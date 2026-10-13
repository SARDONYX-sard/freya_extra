//! Glass material tokens and surfaces shared by the components of this crate.
//!
//! Tuned for dark, translucent themes (see `dark_glass_theme`). The tokens are derived
//! from the active theme where possible, so a theme change still applies.
//!
//! Note: Freya's `EffectExt::blur` blurs the element itself, not what is behind it,
//! so there is no backdrop blur here. Surfaces rely on tint, hairline edges and shadows.

use freya::prelude::*;

pub const RADIUS_SMALL: f32 = 6.0;
pub const RADIUS_MEDIUM: f32 = 10.0;
pub const RADIUS_LARGE: f32 = 14.0;

const fn white(alpha: u8) -> Color {
    Color::from_argb(alpha, 255, 255, 255)
}

const fn black(alpha: u8) -> Color {
    Color::from_argb(alpha, 0, 0, 0)
}

/// How far a surface is lifted from the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elevation {
    /// Recessed track (segmented control, text well).
    Inset,
    /// Panel resting on the window (toolbar, table).
    Raised,
    /// Overlay above everything (menu, drag ghost).
    Floating,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Glass {
    pub accent: Color,
    pub on_accent: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub focus: Color,

    pub surface_inset: Color,
    pub surface_raised: Color,
    pub surface_floating: Color,

    /// 1px edge of panels.
    pub edge: Color,
    /// Edge of small controls such as checkboxes.
    pub control_edge: Color,
    pub separator: Color,
    pub zebra: Color,
    pub hover: Color,
    pub selected: Color,
    pub shadow: Color,
}

/// Read the active theme and derive the glass tokens.
pub fn use_glass() -> Glass {
    let colors = use_theme().read().colors.clone();

    Glass {
        accent: colors.primary,
        on_accent: Color::WHITE,
        text: colors.text_primary,
        text_secondary: colors.text_secondary,
        focus: colors.border_focus,

        surface_inset: black(0x38),
        surface_raised: colors.surface_primary,
        // Without backdrop blur, floating surfaces must be more opaque to stay legible.
        // Same RGB as the `surface_secondary` of `dark_glass_theme`.
        surface_floating: Color::from_argb(0xEB, 0x30, 0x39, 0x46),

        edge: white(0x1F),
        control_edge: white(0x4D),
        separator: white(0x1A),
        zebra: white(0x0A),
        hover: white(0x14),
        selected: white(0x2E),
        shadow: colors.shadow,
    }
}

impl Glass {
    /// A rounded, tinted surface with a hairline edge and an elevation-dependent shadow.
    pub fn panel(&self, elevation: Elevation, radius: f32) -> Rect {
        let (fill, blur) = match elevation {
            Elevation::Inset => (self.surface_inset, 0.0),
            Elevation::Raised => (self.surface_raised, 16.0),
            Elevation::Floating => (self.surface_floating, 28.0),
        };

        let panel = rect()
            .corner_radius(CornerRadius::new_all(radius))
            .background(fill)
            .backdrop_blur(blur) // NOTE: No call blur == 0.0
            .border(Border::new().fill(self.edge).width(1.0));

        match elevation {
            Elevation::Inset => {
                panel.shadow(Shadow::new().inset().y(1.0).blur(2.0).color(black(0x40)))
            }
            Elevation::Raised => panel.shadow(Shadow::new().y(4.0).blur(16.0).color(self.shadow)),
            Elevation::Floating => {
                panel.shadow(Shadow::new().y(12.0).blur(32.0).color(self.shadow))
            }
        }
    }
}
