use freya::{
    animation::{AnimNum, AnimatedValue as _, Ease, Function, OnChange, OnCreation, use_animation},
    prelude::*,
};

use crate::state::ModManagerContext;

#[derive(PartialEq)]
pub struct QuickSearch;

impl Component for QuickSearch {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let mut expanded = use_state(|| false);

        let animation = use_animation(move |conf| {
            conf.on_change(OnChange::Rerun);
            conf.on_creation(OnCreation::Finish);

            let width =
                AnimNum::new(36.0, 300.0).time(200).ease(Ease::Out).function(Function::Expo);

            if expanded() { width } else { width.into_reversed() }
        });

        let width = animation.read().value();
        let search = context.search;

        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(4.0)
            .width(Size::px(width))
            .height(Size::px(36.0))
            .corner_radius(CornerRadius::new_all(6.0))
            .padding(Gaps::new(2.0, 4.0, 2.0, 4.0))
            .maybe_child((expanded() && width > 60.0).then(|| {
                Input::new(search)
                    .placeholder("Search...")
                    .width(Size::px((width - 48.0).max(1.0)))
                    .into_element()
            }))
            .child(Button::new().on_press(move |_| expanded.toggle()).child("⌕"))
    }
}
