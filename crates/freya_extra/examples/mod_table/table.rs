use freya::prelude::*;
use freya_extra::dnd_table::{
    ColumnWidth, DndTable, DndTableBody, DndTableCell, DndTableHead, DndTableRow,
};

use crate::{
    model::{COLUMN_TITLES, Query, build_view, reorder_visible},
    state::ModManagerContext,
    widgets::{context_menu, is_right_click, set_enabled_action},
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
        header_cell(1, false),
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
                    text_cell(1, row.id.to_string(), false),
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
pub struct BulkActions;

impl Component for BulkActions {
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

        let mut menu_items: Vec<(String, Box<dyn FnMut()>)> = vec![
            ("Enable all".to_string(), set_enabled_action(context.rows, None, true)),
            ("Disable all".to_string(), set_enabled_action(context.rows, None, false)),
        ];

        if shown != total {
            menu_items.push((
                format!("Enable visible ({shown})"),
                set_enabled_action(context.rows, Some(visible_ids.clone()), true),
            ));

            menu_items.push((
                format!("Disable visible ({shown})"),
                set_enabled_action(context.rows, Some(visible_ids), false),
            ));
        }

        let menu_position = *context.menu.read();

        let mut root = rect().width(Size::fill()).height(Size::fill());

        if let Some(at) = menu_position {
            root = root.child(context_menu(at, menu_items, context.menu, context.menu_hover));
        }

        root
    }
}
