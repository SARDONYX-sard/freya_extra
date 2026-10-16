use freya::{
    prelude::*,
    torin::prelude::{Area, Size2D},
};
use freya_extra::dropdown_menu::DropdownMenu;

use crate::{
    model::{ColumnFilter, FilterColumn, FilterOperator},
    state::ModManagerContext,
};

#[derive(PartialEq)]
pub struct ColumnFilterEditor;

impl Component for ColumnFilterEditor {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let mut column = use_state(|| FilterColumn::Name);
        let mut operator = use_state(|| FilterOperator::Contains);
        let mut value = use_state(String::new);
        let mut show_filter = use_state(|| false);
        let mut trigger_area = use_state(|| None::<Area>);
        let mut popup_area = use_state(|| None::<Area>);
        let mut popup_measurement = use_state(|| None::<(Area, Size2D)>);

        let selected_column = *column.read();
        let selected_operator = *operator.read();

        let column_menu = DropdownMenu::new(selected_column)
            .item(FilterColumn::Name, "Mod Name")
            .item(FilterColumn::Type, "Type")
            .item(FilterColumn::Site, "URL")
            .item(FilterColumn::Enabled, "Enabled")
            .on_change(move |next| column.set(next));

        let operator_menu = DropdownMenu::new(selected_operator)
            .item(FilterOperator::Contains, "Contains")
            .item(FilterOperator::Equals, "Equals")
            .item(FilterOperator::StartsWith, "Starts with")
            .on_change(move |next| operator.set(next));

        let mut filters = context.filters;
        let current_filters = filters.read().clone();

        let mut filter_ui = rect()
            .spacing(8.0)
            .width(Size::fill())
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .child(column_menu)
                    .child(operator_menu),
            )
            .child(Input::new(value).placeholder("Filter value...").width(Size::fill()))
            .child(
                Button::new()
                    .on_press(move |_| {
                        let filter_value = value.read().trim().to_string();

                        if filter_value.is_empty() {
                            return;
                        }

                        let mut next = filters.read().clone();
                        next.push(ColumnFilter {
                            column: *column.read(),
                            operator: *operator.read(),
                            value: filter_value,
                        });

                        filters.set(next);
                        value.set(String::new());
                    })
                    .child("Add filter"),
            );

        for (index, filter) in current_filters.iter().enumerate() {
            let mut filters = context.filters;

            filter_ui = filter_ui.child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .child(
                        label()
                            .text(format!(
                                "{} {} {}",
                                filter.column.title(),
                                filter.operator.title(),
                                filter.value
                            ))
                            .font_size(12.0),
                    )
                    .child(
                        Button::new()
                            .compact()
                            .on_press(move |_| {
                                let mut next = filters.read().clone();

                                if index < next.len() {
                                    next.remove(index);
                                    filters.set(next);
                                }
                            })
                            .child("Remove"),
                    ),
            );
        }

        let mut show_filter_for_escape = show_filter;
        let mut show_filter_for_close = show_filter;

        let popup = MenuContainer::new().child(
            rect()
                .on_global_pointer_up(move |event: Event<PointerEventData>| {
                    let point = event.global_location();

                    let inside_trigger = trigger_area
                        .read()
                        .is_some_and(|area| contains_point(area, point.x, point.y));

                    let inside_popup = popup_area
                        .read()
                        .is_some_and(|area| contains_point(area, point.x, point.y));

                    if !inside_trigger && !inside_popup {
                        show_filter.set(false);
                    }
                })
                .on_sized(move |event: Event<SizedEventData>| {
                    popup_area.set(Some(event.area));
                })
                .width(Size::px(360.0))
                .padding(12.0)
                .spacing(8.0)
                .corner_radius(8.0)
                .border(Border::new().fill(Color::from_rgb(120, 120, 120)).width(1.0))
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .main_align(Alignment::SpaceBetween)
                        .spacing(8.0)
                        .child(label().text("Filter").font_size(16.0))
                        .child(
                            Button::new()
                                .compact()
                                .on_press(move |_| show_filter_for_close.set(false))
                                .child("✕"),
                        ),
                )
                .child(filter_ui),
        );

        rect().horizontal().cross_align(Alignment::Center).spacing(8.0).child(
            rect()
                .on_sized(move |event: Event<SizedEventData>| {
                    trigger_area.set(Some(event.area));
                })
                .child(
                    Button::new()
                        .on_press(move |_| {
                            show_filter.toggle();
                            popup_measurement.set(None);
                        })
                        .child(format!("Filter ({})", current_filters.len())),
                )
                .maybe_child(show_filter().then(|| {
                    rect()
                        .position(Position::new_absolute().top(40.0))
                        .on_global_key_down(move |event: Event<KeyboardEventData>| {
                            if event.key == Key::Named(NamedKey::Escape) {
                                show_filter_for_escape.set(false);
                            }
                        })
                        .child(popup)
                })),
        )
    }
}

fn contains_point(area: Area, x: f64, y: f64) -> bool {
    let left = area.origin.x as f64;
    let top = area.origin.y as f64;
    let right = left + area.size.width as f64;
    let bottom = top + area.size.height as f64;

    x >= left && x <= right && y >= top && y <= bottom
}
