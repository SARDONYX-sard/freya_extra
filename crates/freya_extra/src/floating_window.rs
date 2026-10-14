//! A small window that floats above the app, similar to `egui::Window`.
//!
//! The user can drag it by the title bar, collapse it down to the title bar and close it.
//!
//! ```ignore
//! let mut open = use_state(|| false);
//!
//! FloatingWindow::new("Settings")
//!     .open(open())
//!     .on_close(move |_| open.set(false))
//!     .width(360.0)
//!     .child("content")
//! ```
//!
//! The window is positioned relative to the app window ([`Position::new_global`]) and painted on
//! the overlay layer. Unlike a `DraggableCanvas` it does not need a parent with a size, so it can
//! be declared anywhere in the tree (for example next to the button that opens it).
//!
//! Keep the component mounted and drive it with [`FloatingWindow::open`]. Its position and
//! collapsed state live inside the component, so they survive closing and reopening.

use freya::prelude::*;

const TITLE_BAR_HEIGHT: f32 = 40.0;
const CORNER_RADIUS: f32 = 10.0;

/// A draggable, collapsible, closable window. See the [module docs](self).
#[derive(Clone)]
pub struct FloatingWindow {
    title: String,
    children: Vec<Element>,
    open: bool,
    initial_position: (f32, f32),
    width: f32,
    body_height: Option<f32>,
    on_close: Option<EventHandler<()>>,
}

impl std::fmt::Debug for FloatingWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FloatingWindow")
            .field("title", &self.title)
            .field("open", &self.open)
            .field("initial_position", &self.initial_position)
            .field("width", &self.width)
            .field("body_height", &self.body_height)
            .finish_non_exhaustive()
    }
}

impl FloatingWindow {
    /// Create a closed window with the given title.
    #[inline]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
            open: false,
            initial_position: (40.0, 40.0),
            width: 420.0,
            body_height: None,
            on_close: None,
        }
    }

    /// Show or hide the window. Hiding keeps its position and collapsed state.
    #[inline]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Position (relative to the app window) the first time the window is shown.
    #[inline]
    pub fn initial_position(mut self, x: f32, y: f32) -> Self {
        self.initial_position = (x, y);
        self
    }

    /// Outer width in pixels.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(1.0);
        self
    }

    /// Fixed height of the content area in pixels. By default it fits the content.
    #[inline]
    pub fn body_height(mut self, height: f32) -> Self {
        self.body_height = Some(height.max(1.0));
        self
    }

    /// Called when the close button is pressed. The owner decides whether to hide the window.
    ///
    /// Takes an [`EventHandler`] like `Button::on_press`, so the closure may be `FnMut`
    /// (for example `move |_| open.set(false)`).
    #[inline]
    pub fn on_close(mut self, callback: impl Into<EventHandler<()>>) -> Self {
        self.on_close = Some(callback.into());
        self
    }

    /// Append content.
    #[inline]
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Append several content elements.
    #[inline]
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }
}

impl PartialEq for FloatingWindow {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.children == other.children
            && self.open == other.open
            && self.initial_position == other.initial_position
            && self.width == other.width
            && self.body_height == other.body_height
            && self.on_close == other.on_close
    }
}

/// Keep the window reachable: fully inside horizontally, and the title bar inside vertically.
fn clamp_position(x: f32, y: f32, width: f32, window: (f32, f32)) -> (f32, f32) {
    let max_x = (window.0 - width).max(0.0);
    let max_y = (window.1 - TITLE_BAR_HEIGHT).max(0.0);

    (x.clamp(0.0, max_x), y.clamp(0.0, max_y))
}

impl Component for FloatingWindow {
    fn render(&self) -> impl IntoElement {
        // Hooks must run in the same order on every render, also while closed.
        let mut position = use_state(|| self.initial_position);
        // Offset from the window's top-left corner to the pointer while dragging.
        let mut grab = use_state(|| None::<(f32, f32)>);
        let mut collapsed = use_state(|| false);
        let window_size = Platform::get().current_window().root_size;
        // Also a hook: it must run before the early return below.
        let colors = use_theme().read().colors.clone();

        if !self.open {
            // A closed window must not keep a drag alive.
            if grab.read().is_some() {
                grab.set(None);
            }

            return Element::from(rect().width(Size::px(0.0)).height(Size::px(0.0)));
        }

        let width = self.width;
        let is_collapsed = *collapsed.read();
        let is_dragging = grab.read().is_some();
        let (left, top) = *position.read();

        let on_title_down = move |event: Event<PointerEventData>| {
            if !event.data().is_primary() {
                return;
            }

            let (pointer_x, pointer_y) = event.global_location().to_tuple();
            let (x, y) = *position.read();

            grab.set(Some((pointer_x as f32 - x, pointer_y as f32 - y)));
            event.stop_propagation();
        };

        let on_global_pointer_move = move |event: Event<PointerEventData>| {
            let Some((grab_x, grab_y)) = *grab.read() else {
                return;
            };

            let (pointer_x, pointer_y) = event.global_location().to_tuple();
            let size = *window_size.read();

            position.set(clamp_position(
                pointer_x as f32 - grab_x,
                pointer_y as f32 - grab_y,
                width,
                (size.width, size.height),
            ));
        };

        let on_global_pointer_up = move |_| {
            if grab.read().is_some() {
                grab.set(None);
            }
        };

        let on_close = self.on_close.clone();

        // `Size::fill()` takes all the remaining width in torin, not a share of it: a filled title
        // followed by buttons leaves them 0px and their labels wrap one character per line.
        // A flex row shares the width properly.
        let title_bar = rect()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(TITLE_BAR_HEIGHT))
            .padding((0.0, 8.0, 0.0, 12.0))
            .spacing(4.0)
            .child(
                // Only the title area drags the window, so the buttons keep working.
                rect()
                    .width(Size::flex(1.0))
                    .height(Size::fill())
                    .main_align(Alignment::Center)
                    .cursor(if is_dragging { CursorIcon::Grabbing } else { CursorIcon::Grab })
                    .on_pointer_down(on_title_down)
                    .child(
                        label()
                            .text(self.title.clone())
                            .font_size(15.0)
                            .font_weight(FontWeight::BOLD)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    ),
            )
            .child(
                Button::new()
                    .compact()
                    .on_press(move |_| collapsed.set(!is_collapsed))
                    .child(if is_collapsed { "Expand" } else { "Collapse" }),
            )
            .child(
                Button::new()
                    .compact()
                    .on_press(move |_| {
                        if let Some(on_close) = &on_close {
                            on_close.call(());
                        }
                    })
                    .child("Close"),
            );

        let mut panel = rect()
            .vertical()
            .width(Size::px(width))
            .spacing(0.0)
            .background(colors.background)
            .color(colors.text_primary)
            .corner_radius(CornerRadius::new_all(CORNER_RADIUS))
            .border(Border::new().fill(colors.border).width(1.0))
            .shadow(Shadow::new().y(8.0).blur(24.0).color(colors.shadow))
            .child(title_bar);

        if !is_collapsed {
            let mut body = rect().width(Size::fill()).padding(12.0).spacing(8.0);

            if let Some(height) = self.body_height {
                body = body.height(Size::px(height));
            }

            panel = panel.child(body.children(self.children.clone()));
        }

        // Zero-sized anchor: the window itself is positioned globally and does not
        // take part in the layout of the parent.
        Element::from(
            rect().width(Size::px(0.0)).height(Size::px(0.0)).child(
                rect()
                    .position(Position::new_global().left(left).top(top))
                    .layer(Layer::Overlay)
                    .on_global_pointer_move(on_global_pointer_move)
                    .on_global_pointer_up(on_global_pointer_up)
                    .child(panel),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_the_window_reachable() {
        let window = (800.0, 600.0);

        assert_eq!(clamp_position(100.0, 50.0, 420.0, window), (100.0, 50.0));
        assert_eq!(clamp_position(-30.0, -10.0, 420.0, window), (0.0, 0.0));
        assert_eq!(clamp_position(900.0, 900.0, 420.0, window), (380.0, 560.0));
        // Narrower app window than the floating window: pin to the left edge.
        assert_eq!(clamp_position(50.0, 0.0, 420.0, (300.0, 600.0)), (0.0, 0.0));
    }
}
