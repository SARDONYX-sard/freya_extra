use freya::prelude::*;
use freya_extra::dnd_table::{
    ColumnWidth, DndTable, DndTableBody, DndTableCell, DndTableHead, DndTableRow,
};

use crate::{
    model::{COLUMN_TITLES, ModRow, Query, apply_enabled, build_view, reorder_visible},
    state::ModManagerContext,
};

fn header_cell(column: usize, resizable: bool) -> Element {
    DndTableCell::new()
        .column(column)
        .header(true)
        .title(COLUMN_TITLES[column])
        .sortable(true)
        .resizable(resizable)
        .into_element()
}

fn table_head() -> DndTableHead {
    DndTableHead::new().child(DndTableRow::new().header(true).children([
        header_cell(0, false),
        header_cell(1, true),
        header_cell(2, true),
        header_cell(3, true),
        header_cell(4, true),
        header_cell(5, false),
    ]))
}

fn text_cell(column: usize, text: impl Into<String>, resizable: bool) -> Element {
    DndTableCell::new()
        .column(column)
        .resizable(resizable)
        .child(
            label()
                .text(text.into())
                .font_size(13.0)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis),
        )
        .into_element()
}

fn enabled_cell(id: usize, enabled: bool, context: ModManagerContext) -> Element {
    let mut rows = context.rows;

    DndTableCell::new()
        .column(0)
        .child(
            rect()
                .height(Size::fill())
                .cross_align(Alignment::Center)
                .child(Checkbox::new().selected(enabled).size(16.))
                .on_focus_press(move |_| {
                    let mut next = rows.read().clone();

                    if let Some(row) = next.iter_mut().find(|row| row.id == id) {
                        row.enabled = !row.enabled;
                        rows.set(next);
                    }
                }),
        )
        .into_element()
}

fn table_body(view: &[&crate::model::ModRow], context: ModManagerContext) -> DndTableBody {
    view.iter().enumerate().fold(DndTableBody::new(), |body, (index, row)| {
        body.row(
            row.id,
            DndTableRow::new()
                .id(row.id)
                .position(index)
                .children([
                    enabled_cell(row.id, row.enabled, context),
                    text_cell(1, row.id.to_string(), true),
                    text_cell(2, row.name.clone(), true),
                    text_cell(3, row.mod_type.clone(), true),
                    text_cell(4, row.site.clone(), true),
                    text_cell(5, row.priority.to_string(), false),
                ])
                .into_element(),
        )
    })
}

#[derive(PartialEq)]
pub struct ModTable;

impl Component for ModTable {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();

        let all = context.rows.read().clone();

        let query = Query {
            text: context.search.read().trim().to_lowercase(),
            filters: context.filters.read().clone(),
            enabled_only: *context.enabled_only.read(),
            sort: *context.sort.read(),
        };

        let view = build_view(&all, &query);
        let visible_ids: Vec<usize> = view.iter().map(|row| row.id).collect();

        let shown = view.len();
        let total = all.len();

        let rows_for_reorder = context.rows;
        let sort_for_callback = context.sort;

        let table = DndTable::new()
            .columns([
                ColumnWidth::Ratio(0.07),
                ColumnWidth::Ratio(0.05),
                ColumnWidth::Ratio(0.27),
                ColumnWidth::Ratio(0.12),
                ColumnWidth::Ratio(0.27),
                ColumnWidth::Fill,
            ])
            .default_sort(Some((5, true)))
            .density(*context.density.read())
            .max_height(555.0)
            .sort(query.sort)
            .on_sort(move |next| {
                let mut state = sort_for_callback;
                state.set(next);
            })
            .on_reorder(move |moved_id, target_id, before| {
                let mut next = rows_for_reorder.read().clone();

                if reorder_visible(&mut next, &visible_ids, moved_id, target_id, before) {
                    let mut state = rows_for_reorder;
                    state.set(next);
                }
            })
            .child(table_head())
            .child(table_body(&view, context));

        let menu = context.menu;

        let table_area = rect()
            .on_pointer_down(move |event: Event<PointerEventData>| {
                if is_right_click(&event) {
                    let (x, y) = event.global_location().to_tuple();
                    let mut state = menu;
                    state.set(Some((x, y)));
                }
            })
            .child(table);

        rect()
            .spacing(12.0)
            .child(
                label()
                    .text(format!(
                        "Drag a row onto another to change priority. \
                         Right-click for bulk actions. ({shown} of {total} shown)"
                    ))
                    .font_size(12.0),
            )
            .child(table_area)
    }
}

#[derive(PartialEq)]
pub struct TableContextMenu;

impl Component for TableContextMenu {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let mut menu = context.menu;
        let hover = context.menu_hover;
        let popup_area = use_state(|| None::<Area>);

        let Some((x, y)) = *menu.read() else {
            return rect().into_element();
        };

        let rows = context.rows;
        let all = rows.read().clone();

        let query = Query {
            text: context.search.read().trim().to_lowercase(),
            filters: context.filters.read().clone(),
            enabled_only: *context.enabled_only.read(),
            sort: *context.sort.read(),
        };

        let view = build_view(&all, &query);
        let visible_ids: Vec<usize> = view.iter().map(|row| row.id).collect();

        let shown = view.len();
        let total = all.len();

        let mut menu_items: Vec<(String, Box<dyn FnMut()>)> = vec![
            ("Enable all".to_string(), set_enabled_action(rows, None, true)),
            ("Disable all".to_string(), set_enabled_action(rows, None, false)),
        ];

        if shown != total {
            menu_items.push((
                format!("Enable visible ({shown})"),
                set_enabled_action(rows, Some(visible_ids.clone()), true),
            ));
            menu_items.push((
                format!("Disable visible ({shown})"),
                set_enabled_action(rows, Some(visible_ids), false),
            ));
        }

        let mut entries: Vec<Element> = Vec::with_capacity(menu_items.len() + 1);

        for (title, mut action) in menu_items {
            let mut close_menu = menu;
            let mut reset_hover = hover;

            entries.push(
                MenuItem::new()
                    .theme(MenuItemThemePartial {
                        hover_background: Some(Preference::Reference("active")),
                        ..Default::default()
                    })
                    .on_press(move |_| {
                        action();
                        reset_hover.set(None);
                        close_menu.set(None);
                    })
                    .child(title)
                    .into_element(),
            );
        }

        let mut close_on_outside = menu;
        let mut reset_hover = hover;
        let mut measure_popup = popup_area;

        rect()
            .position(Position::new_absolute().left(x as f32).top(y as f32))
            .width(Size::fill())
            .height(Size::fill())
            .layer(1000)
            .on_global_pointer_down(move |event: Event<PointerEventData>| {
                if is_right_click(&event) && popup_area.read().is_none() {
                    let point = event.global_location();
                    menu.set(Some((point.x, point.y)));
                }
            })
            .on_global_pointer_up(move |event: Event<PointerEventData>| {
                let point = event.global_location();

                let inside_popup =
                    popup_area.read().is_some_and(|area| contains_point(area, point.x, point.y));

                if !inside_popup {
                    reset_hover.set(None);
                    close_on_outside.set(None);
                }
            })
            .child(
                rect()
                    .position(Position::new_global().top(y as f32).left(x as f32))
                    .on_sized(move |event: Event<SizedEventData>| {
                        measure_popup.set(Some(event.area));
                    })
                    .child(Menu::new().children(entries)),
            )
            .into_element()
    }
}

fn set_enabled_action(
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

fn contains_point(area: Area, x: f64, y: f64) -> bool {
    let left = area.origin.x as f64;
    let top = area.origin.y as f64;
    let right = left + area.size.width as f64;
    let bottom = top + area.size.height as f64;

    x >= left && x <= right && y >= top && y <= bottom
}

fn is_right_click(event: &Event<PointerEventData>) -> bool {
    event.button() == Some(MouseButton::Right)
}
