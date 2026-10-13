use freya::{
    animation::{AnimNum, Ease, use_animation},
    prelude::*,
};

/// ```rust
/// use freya::prelude::*;
///
/// let density_select = DropdownMenu::new(current_density)
/// .item(Density::Compact, "Compact")
/// .item(Density::Standard, "Standard")
/// .item(Density::Comfortable, "Comfortable")
/// .on_change(move |value| {
///     let mut density = density;
///     density.set(value);
/// });
/// ```
pub struct DropdownMenu<T> {
    placeholder: String,
    selected: T,
    items: Vec<(T, String)>,
    on_change: Option<EventHandler<T>>,
    menu_theme: Option<MenuContainerThemePartial>,
    item_theme: Option<MenuItemThemePartial>,
    position: Position,
}

impl<T> PartialEq for DropdownMenu<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.placeholder == other.placeholder
            && self.selected == other.selected
            && self.items == other.items
            && self.menu_theme == other.menu_theme
            && self.item_theme == other.item_theme
    }
}

impl<T> DropdownMenu<T> {
    /// Create a dropdown menu with a label and an initially selected value.
    #[inline]
    pub fn new(selected: T) -> Self {
        Self {
            placeholder: "Select...".to_string(),
            selected,
            items: Vec::new(),
            on_change: None,
            menu_theme: None,
            item_theme: None,
            position: Position::new_absolute().top(40.0),
        }
    }

    /// Set the currently selected value.
    ///
    /// The parent component remains responsible for updating this value.
    #[inline]
    pub fn selected(mut self, selected: T) -> Self {
        self.selected = selected;
        self
    }

    /// Set the placeholder displayed when no item matches the selected value.
    #[inline]
    pub fn placeholder<I>(mut self, placeholder: I) -> Self
    where
        I: Into<String>,
    {
        self.placeholder = placeholder.into();
        self
    }

    /// Append an item to the dropdown menu.
    #[inline]
    pub fn item<I>(mut self, value: T, label: I) -> Self
    where
        I: Into<String>,
    {
        self.items.push((value, label.into()));
        self
    }

    /// Replace all dropdown items.
    #[inline]
    pub fn items(mut self, items: impl IntoIterator<Item = (T, String)>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    /// Set the callback invoked when a different item is selected.
    ///
    /// The callback receives the selected value. The parent component
    /// is responsible for updating the value passed to [`Self::selected`].
    #[inline]
    pub fn on_change(mut self, callback: impl Into<EventHandler<T>>) -> Self {
        self.on_change = Some(callback.into());
        self
    }

    /// Set dropdown menu position.(Default: absolute top 40.0)
    #[inline]
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Set the theme used by the menu container.
    #[inline]
    pub fn menu_theme(mut self, theme: MenuContainerThemePartial) -> Self {
        self.menu_theme = Some(theme);
        self
    }

    /// Set the theme used by menu items.
    #[inline]
    pub fn item_theme(mut self, theme: MenuItemThemePartial) -> Self {
        self.item_theme = Some(theme);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MenuState {
    Closed,
    Opening,
    Open,
    Closing,
}

impl MenuState {
    fn is_visible(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    fn begin_open(&mut self) -> bool {
        match self {
            Self::Closed | Self::Closing => {
                *self = Self::Opening;
                true
            }
            Self::Opening | Self::Open => false,
        }
    }

    fn begin_close(&mut self) -> bool {
        match self {
            Self::Opening | Self::Open => {
                *self = Self::Closing;
                true
            }
            Self::Closed | Self::Closing => false,
        }
    }
}

impl<T> Component for DropdownMenu<T>
where
    T: Clone + PartialEq + 'static,
{
    fn render(&self) -> impl IntoElement {
        let mut menu_state = use_state(|| MenuState::Closed);
        let mut open_anim = use_animation(|_| AnimNum::new(0.5, 1.0).time(80).ease(Ease::Out));
        let mut close_anim = use_animation(|_| AnimNum::new(1.0, 0.5).time(80).ease(Ease::Out));

        let mut begin_open = {
            let mut menu_state = menu_state;
            move || {
                if menu_state().begin_open() {
                    menu_state.set(MenuState::Opening);
                    open_anim.start();
                }
            }
        };

        let mut begin_close = {
            let mut menu_state = menu_state;
            move || {
                if menu_state().begin_close() {
                    menu_state.set(MenuState::Closing);
                    close_anim.start();
                }
            }
        };

        use_side_effect(move || {
            if !*open_anim.is_running().read() && menu_state() == MenuState::Opening {
                menu_state.set(MenuState::Open);
            }

            if !*close_anim.is_running().read() && menu_state() == MenuState::Closing {
                menu_state.set(MenuState::Closed);
            }
        });

        let state = menu_state();
        let is_visible = state.is_visible();

        let scale = match state {
            MenuState::Opening => open_anim.read().value(),
            MenuState::Closing => close_anim.read().value(),
            MenuState::Open | MenuState::Closed => 1.0,
        };

        let selected_label = self
            .items
            .iter()
            .find(|(value, _)| value == &self.selected)
            .map(|(_, label)| label.as_str())
            .unwrap_or(&self.placeholder);

        let selected = self.selected.clone();
        let callback = self.on_change.clone();
        let item_theme = self.item_theme.clone();

        let menu = self.items.iter().fold(
            Menu::new()
                .on_close(move |_| begin_close())
                .on_escape(move |_| begin_close())
                .map(self.menu_theme.clone(), |menu, theme| menu.theme(theme)),
            |menu, (value, label)| {
                let value = value.clone();
                let callback = callback.clone();
                let item_theme = item_theme.clone();
                let is_selected = value == selected;

                menu.child(
                    MenuItem::new()
                        .selected(is_selected)
                        .map(item_theme, |item, theme| item.theme(theme))
                        .on_press(move |_| {
                            if !is_selected && let Some(callback) = &callback {
                                callback.call(value.clone());
                            }

                            begin_close();
                        })
                        .child(
                            rect()
                                .horizontal()
                                .child(rect().width(Size::px(20.0)).child(if is_selected {
                                    "✓"
                                } else {
                                    ""
                                }))
                                .child(label.clone()),
                        ),
                )
            },
        );

        rect()
            .child(
                Button::new()
                    .on_press(move |_| match menu_state() {
                        MenuState::Closed | MenuState::Closing => begin_open(),
                        MenuState::Opening | MenuState::Open => begin_close(),
                    })
                    .child(format!("{selected_label} ▾")),
            )
            .maybe_child(
                is_visible.then(|| rect().scale(scale).position(self.position.clone()).child(menu)),
            )
    }
}
