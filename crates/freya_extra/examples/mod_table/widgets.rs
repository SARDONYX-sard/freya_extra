use freya::prelude::*;

use crate::model::{ModRow, apply_enabled};
use crate::state::ModManagerContext;
use freya_extra::{acrylic, dropdown_menu::DropdownMenu, themes};

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

pub fn is_right_click(event: &Event<PointerEventData>) -> bool {
    if event.button() == Some(MouseButton::Right) {
        println!("Context menu requested");
        return true;
    }
    false
}

pub fn set_enabled_action(
    rows: State<Vec<ModRow>>,
    scope: Option<Vec<usize>>,
    enabled: bool,
) -> Box<dyn FnMut()> {
    Box::new(move || {
        let mut state = rows;
        let mut next = state.read().clone();

        apply_enabled(&mut next, scope.as_deref(), enabled);
        state.set(next);
    })
}

pub fn context_menu(
    at: (f64, f64),
    items: Vec<(String, Box<dyn FnMut()>)>,
    menu: State<Option<(f64, f64)>>,
    hover: State<Option<usize>>,
) -> Element {
    let total = items.len();
    let mut entries: Vec<Element> = Vec::with_capacity(total + 1);

    for (index, (title, mut action)) in items.into_iter().enumerate() {
        let hovered = *hover.read() == Some(index);

        let mut enter = hover;
        let mut leave = hover;
        let mut reset = hover;
        let mut close = menu;

        let item = rect()
            .width(Size::fill())
            .padding(Gaps::new(5.0, 10.0, 5.0, 10.0))
            .corner_radius(CornerRadius::new_all(5.0))
            .on_pointer_enter(move |_| enter.set(Some(index)))
            .on_pointer_leave(move |_| {
                if *leave.read() == Some(index) {
                    leave.set(None);
                }
            })
            .on_mouse_up(move |_| {
                action();
                reset.set(None);
                close.set(None);
            });

        if hovered {
            // item = item.background(glass.accent);
        }

        entries.push(item.child(label().text(title).font_size(13.0)).into());

        if index == 1 && total > 2 {
            entries.push(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.0))
                    .margin(Gaps::new(4.0, 6.0, 4.0, 6.0))
                    .into(),
            );
        }
    }

    let panel = rect()
        .position(Position::new_absolute().top(at.1 as f32).left(at.0 as f32))
        .width(Size::px(220.0))
        .padding(4.0)
        .on_pointer_down(|event: Event<PointerEventData>| {
            event.stop_propagation();
        })
        .children(entries);

    let mut close = menu;
    let mut reset = hover;

    rect()
        .position(Position::new_absolute().top(0.0).left(0.0))
        .width(Size::fill())
        .height(Size::fill())
        .layer(1000)
        .on_pointer_down(move |_| {
            reset.set(None);
            close.set(None);
        })
        .child(panel)
        .into()
}
