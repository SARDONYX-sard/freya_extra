// SPDX-FileCopyrightText: (C) 2026 Marc Espín Sanz
// SPDX-License-Identifier: MIT
//
// Fork rationale:
// - Alpha cannot be entered directly at this stage.
// - Keep ContextMenuViewer because omitting it causes a panic when clicked
//   due to the missing ViewContext.
//
// ref: index.crates.io-1949cf8c6b5b557f/freya-components-0.5.0-rc.10/src/color_picker.rs
use freya::{
    animation::{
        AnimNum, AnimatedValue as _, Ease, Function, OnChange, OnCreation, ReadAnimatedValue as _,
        use_animation,
    },
    prelude::*,
    torin::geometry::Point2D,
};

/// HSV-based gradient color picker.(With alpha)
///
/// ## Example
///
/// ```rust
/// # use freya_extra::color_picker::ColorPicker;
/// fn app() -> impl IntoElement {
///     let mut color = use_state(|| Color::from_hsv(0.0, 1.0, 1.0));
///     rect()
///         .padding(6.)
///         .child(ColorPicker::new(move |c| color.set(c)).value(color()))
/// }
/// ```
#[derive(Clone, PartialEq)]
pub struct ColorPicker {
    pub(crate) theme: Option<ColorPickerThemePartial>,
    value: Color,
    on_change: EventHandler<Color>,
    width: Size,
    key: DiffKey,
}

impl KeyExt for ColorPicker {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl ColorPicker {
    pub fn new(on_change: impl Into<EventHandler<Color>>) -> Self {
        Self {
            theme: None,
            value: Color::WHITE,
            on_change: on_change.into(),
            width: Size::px(220.),
            key: DiffKey::None,
        }
    }

    pub fn value(mut self, value: Color) -> Self {
        self.value = value;
        self
    }

    pub fn width(mut self, width: impl Into<Size>) -> Self {
        self.width = width.into();
        self
    }
}

/// Which part of the color picker is being dragged, if any.
#[derive(Clone, Copy, PartialEq, Default)]
enum DragTarget {
    #[default]
    None,
    Sv,
    Hue,
    Alpha,
}

impl Component for ColorPicker {
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| false);
        let mut color = use_state(|| self.value);
        let mut dragging = use_state(DragTarget::default);
        let mut area = use_state(Area::default);
        let mut alpha_area = use_state(Area::default);
        let mut hue_area = use_state(Area::default);
        let mut preview_area = use_state(|| None::<Area>);
        let mut popup_size = use_state(|| None::<Size2D>);

        let theme = get_theme!(&self.theme, ColorPickerThemePreference, "color_picker");
        let is_open = open();

        let preview = rect()
            .width(Size::px(40.))
            .height(Size::px(24.))
            .corner_radius(4.)
            .background(self.value)
            .border(
                Border::new().fill(theme.border_fill).width(1.).alignment(BorderAlignment::Inner),
            )
            .on_sized(move |e: Event<SizedEventData>| preview_area.set_if_modified(Some(e.area)))
            .on_press(move |_| {
                open.toggle();
            });

        let hue_bar = {
            let hsv = color.read().to_hsv();
            let marker_width = 3.0_f32;
            let bar_width = hue_area.read().to_f64().width() as f32;
            let hue_ratio = hsv.h.rem_euclid(360.0) / 360.0;
            let marker_x = (bar_width * hue_ratio - marker_width / 2.0)
                .clamp(0.0, (bar_width - marker_width).max(0.0));

            rect()
                .height(Size::px(18.))
                .width(Size::fill())
                .corner_radius(4.)
                .overflow(Overflow::Clip)
                .on_sized(move |e: Event<SizedEventData>| hue_area.set(e.area))
                .background(
                    LinearGradient::new()
                        .angle(-90.)
                        .stop(((255, 0, 0), 0.))
                        .stop(((255, 255, 0), 16.))
                        .stop(((0, 255, 0), 33.))
                        .stop(((0, 255, 255), 50.))
                        .stop(((0, 0, 255), 66.))
                        .stop(((255, 0, 255), 83.))
                        .stop(((255, 0, 0), 100.)),
                )
                .child(
                    rect()
                        .position(Position::new_absolute())
                        .width(Size::px(marker_width))
                        .height(Size::px(18.))
                        .offset_x(marker_x)
                        .child(rect().expanded().background(Color::from_argb(180, 255, 255, 255))),
                )
        };

        let alpha_bar = {
            let current = *color.read();
            let alpha_ratio = current.a() as f32 / 255.0;
            let bar_width = alpha_area.read().to_f64().width() as f32;
            let bar_height = 18.0_f32;
            let marker_width = 3.0_f32;

            let marker_x = (bar_width * alpha_ratio - marker_width / 2.0)
                .clamp(0.0, (bar_width - marker_width).max(0.0));

            let line_y = bar_height / 2.0 - 1.0;

            rect()
                .height(Size::px(18.))
                .width(Size::fill())
                .corner_radius(4.)
                .overflow(Overflow::Clip)
                .on_sized(move |e: Event<SizedEventData>| {
                    alpha_area.set(e.area);
                })
                .background((190, 190, 190))
                .child(
                    rect().expanded().background(
                        LinearGradient::new()
                            .angle(-90.)
                            .stop((Color::from_argb(0, 0, 0, 0), 0.))
                            .stop((Color::from_argb(255, 0, 0, 0), 100.)),
                    ),
                )
                .child(
                    rect()
                        .position(Position::new_absolute())
                        .width(Size::fill())
                        .height(Size::px(2.))
                        .offset_y(line_y)
                        .child(rect().expanded().background(Color::from_argb(110, 255, 255, 255))),
                )
                .child(
                    rect()
                        .position(Position::new_absolute())
                        .width(Size::px(marker_width))
                        .height(Size::px(bar_height))
                        .offset_x(marker_x)
                        .child(rect().expanded().background(Color::from_argb(220, 255, 255, 255))),
                )
        };

        let sv_area = {
            let hsv = color.read().to_hsv();

            let current_area = area.read().to_f64();
            let sv_width = current_area.width();
            let sv_height = current_area.height();
            let cursor_size = 14.0_f64;

            let cursor_x = (hsv.s as f64 * sv_width - cursor_size / 2.0)
                .clamp(0.0, (sv_width - cursor_size).max(0.0)) as f32;

            let cursor_y = ((1.0 - hsv.v as f64) * sv_height - cursor_size / 2.0)
                .clamp(0.0, (sv_height - cursor_size).max(0.0)) as f32;

            rect()
                .height(Size::px(140.))
                .width(Size::fill())
                .corner_radius(4.)
                .overflow(Overflow::Clip)
                .child(
                    rect()
                        .expanded()
                        .background(
                            LinearGradient::new()
                                .angle(-90.)
                                .stop(((255, 255, 255), 0.))
                                .stop((Color::from_hsv(hsv.h, 1.0, 1.0), 100.)),
                        )
                        .child(
                            rect().position(Position::new_absolute()).expanded().background(
                                LinearGradient::new()
                                    .stop(((255, 255, 255, 0.0), 0.))
                                    .stop(((0, 0, 0), 100.)),
                            ),
                        ),
                )
                .child(
                    rect()
                        .position(Position::new_absolute())
                        .offset_x(cursor_x)
                        .offset_y(cursor_y)
                        .width(Size::px(cursor_size as f32))
                        .height(Size::px(cursor_size as f32))
                        .corner_radius(cursor_size as f32 / 2.0)
                        .background(Color::WHITE)
                        .padding(2.)
                        .child(
                            rect()
                                .expanded()
                                .corner_radius(cursor_size as f32 / 2.0)
                                .background(Color::BLACK),
                        ),
                )
        };

        let mut update_sv = {
            let on_change = self.on_change.clone();
            move |coords: CursorPoint| {
                let sv_area = area.read().to_f64();

                // NOTE: Avoid division by zero when the SV area has not been laid out.
                let width = sv_area.width();
                let height = sv_area.height();
                if width <= 0.0 || height <= 0.0 {
                    return;
                }

                let sat = ((coords.x - sv_area.min_x()) / width).clamp(0.0, 1.0) as f32;
                let rel_y = ((coords.y - sv_area.min_y()) / height).clamp(0.0, 1.0) as f32;
                let v = 1.0 - rel_y;

                let current = *color.read();
                let hsv = current.to_hsv();
                let rgb = Color::from_hsv(hsv.h, sat, v);
                let new_color = Color::from_argb(current.a(), rgb.r(), rgb.g(), rgb.b());

                color.set_if_modified_and_then(new_color, || on_change.call(new_color));
            }
        };

        let mut update_hue = {
            let on_change = self.on_change.clone();
            move |coords: CursorPoint| {
                let bar_area = hue_area.read().to_f64();
                let width = bar_area.width();

                // NOTE: Avoid division by zero when the hue bar has not been laid out.
                if width <= 0.0 {
                    return;
                }

                let rel_x = ((coords.x - bar_area.min_x()) / width).clamp(0.0, 1.0) as f32;
                let current = *color.read();
                let hsv = current.to_hsv();

                // NOTE: Use full saturation when the current color is grayscale,
                // because hue changes are invisible when saturation is zero.
                let saturation = if hsv.s == 0.0 { 1.0 } else { hsv.s };

                // NOTE: Clamp hue to 359 degrees to prevent wrapping at the 360-degree boundary.
                let hue = (rel_x * 360.0).min(359.0);

                let rgb = Color::from_hsv(hue, saturation, hsv.v);
                let new_color = Color::from_argb(current.a(), rgb.r(), rgb.g(), rgb.b());

                color.set_if_modified_and_then(new_color, || on_change.call(new_color));
            }
        };

        let mut update_alpha = {
            let on_change = self.on_change.clone();
            move |coords: CursorPoint| {
                let bar_area = alpha_area.read().to_f64();
                let width = bar_area.width();

                if width <= 0.0 {
                    return;
                }

                let rel_x = ((coords.x - bar_area.min_x()) / width).clamp(0.0, 1.0);
                let alpha = (rel_x * 255.0).round() as u8;

                let current = *color.read();
                let new_color = Color::from_argb(alpha, current.r(), current.g(), current.b());

                color.set_if_modified_and_then(new_color, || on_change.call(new_color));
            }
        };

        let on_sv_pointer_down = {
            let mut update_sv = update_sv.clone();
            move |e: Event<PointerEventData>| {
                if !e.data().is_primary() {
                    return;
                }
                dragging.set(DragTarget::Sv);
                update_sv(e.global_location());
                e.stop_propagation();
                e.prevent_default();
            }
        };

        let on_hue_pointer_down = {
            let mut update_hue = update_hue.clone();
            move |e: Event<PointerEventData>| {
                if !e.data().is_primary() {
                    return;
                }
                dragging.set(DragTarget::Hue);
                update_hue(e.global_location());
                e.stop_propagation();
                e.prevent_default();
            }
        };

        let on_alpha_pointer_down = {
            let mut update_alpha = update_alpha.clone();
            move |e: Event<PointerEventData>| {
                if !e.data().is_primary() {
                    return;
                }
                dragging.set(DragTarget::Alpha);
                update_alpha(e.global_location());
                e.stop_propagation();
                e.prevent_default();
            }
        };

        let on_global_pointer_move = move |e: Event<PointerEventData>| match *dragging.read() {
            DragTarget::Sv => update_sv(e.global_location()),
            DragTarget::Hue => update_hue(e.global_location()),
            DragTarget::Alpha => update_alpha(e.global_location()),
            DragTarget::None => {}
        };

        let on_global_pointer_up = move |_| {
            // Only close the popup if it wasn't being dragged and it is open.
            if is_open && dragging() == DragTarget::None {
                open.set(false);
            }
            dragging.set_if_modified(DragTarget::None);
        };

        let animation = use_animation(move |conf| {
            conf.on_change(OnChange::Rerun);
            conf.on_creation(OnCreation::Finish);

            let scale = AnimNum::new(0.8, 1.).time(200).ease(Ease::Out).function(Function::Expo);
            let opacity = AnimNum::new(0., 1.).time(200).ease(Ease::Out).function(Function::Expo);

            if open() { (scale, opacity) } else { (scale, opacity).into_reversed() }
        });

        let (scale, opacity) = animation.read().value();

        let (offset_x, offset_y, opacity) = match (preview_area(), popup_size()) {
            (Some(preview), Some(size)) => {
                let window = Area::from_size(*Platform::get().current_window().root_size.peek());
                let popup = Area::new(Point2D::new(preview.max_x(), preview.min_y()), size);

                let target = if popup.max_x() > window.max_x() && preview.min_x() >= size.width {
                    Point2D::new(preview.min_x() - size.width, popup.min_y())
                } else {
                    popup.origin
                };

                let clamped = target.clamp(window.origin, window.max() - size.to_vector());
                let offset = clamped - popup.origin;

                (offset.x, offset.y, opacity)
            }
            _ => (0., 0., 0.),
        };

        let popup = rect()
            .on_global_pointer_move(on_global_pointer_move)
            .on_global_pointer_up(on_global_pointer_up)
            .width(self.width.clone())
            .padding(8.)
            .corner_radius(6.)
            .background(theme.background)
            .border(
                Border::new().fill(theme.border_fill).width(1.).alignment(BorderAlignment::Inner),
            )
            .color(theme.color)
            .spacing(8.)
            .shadow(Shadow::new().x(0.).y(2.).blur(8.).color((0, 0, 0, 0.1)))
            .child(
                rect()
                    .on_sized(move |e: Event<SizedEventData>| area.set(e.area))
                    .on_pointer_down(on_sv_pointer_down)
                    .child(sv_area),
            )
            .child(rect().height(Size::px(18.)).on_pointer_down(on_hue_pointer_down).child(hue_bar))
            .child(
                rect()
                    .height(Size::px(18.))
                    .on_pointer_down(on_alpha_pointer_down)
                    .child(alpha_bar),
            )
            .child({
                let current = *color.read();
                let hex = current.to_hex_string();
                let alpha_percent = (current.a() as f64 / 255.0 * 100.0).round() as u32;

                rect()
                    .horizontal()
                    .width(Size::fill())
                    .main_align(Alignment::center())
                    .spacing(8.)
                    .child(
                        Button::new()
                            .on_press(move |e: Event<PressEventData>| {
                                e.stop_propagation();
                                e.prevent_default();

                                if ContextMenu::is_open() {
                                    ContextMenu::close();
                                } else {
                                    ContextMenu::open(
                                        Menu::new()
                                            .child(
                                                MenuButton::new()
                                                    .on_press(move |e: Event<PressEventData>| {
                                                        e.stop_propagation();
                                                        e.prevent_default();
                                                        ContextMenu::close();
                                                        let _ =
                                                            Clipboard::set(color().to_rgb_string());
                                                    })
                                                    .child("Copy as RGBA"),
                                            )
                                            .child(
                                                MenuButton::new()
                                                    .on_press(move |e: Event<PressEventData>| {
                                                        e.stop_propagation();
                                                        e.prevent_default();
                                                        ContextMenu::close();
                                                        let _ =
                                                            Clipboard::set(color().to_hex_string());
                                                    })
                                                    .child("Copy as HEX"),
                                            ),
                                    )
                                }
                            })
                            .compact()
                            .child(format!("{hex}  {alpha_percent}%")),
                    )
            });

        rect().child(ContextMenuViewer::new()).child(
            rect().horizontal().child(preview).maybe_child((is_open || opacity > 0.).then(|| {
                rect()
                    .layer(Layer::Overlay)
                    .width(Size::px(0.))
                    .height(Size::px(0.))
                    .offset_x(offset_x)
                    .offset_y(offset_y)
                    .opacity(opacity)
                    .child(popup.scale(scale).on_sized(move |e: Event<SizedEventData>| {
                        popup_size.set_if_modified(Some(e.area.size))
                    }))
            })),
        )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
