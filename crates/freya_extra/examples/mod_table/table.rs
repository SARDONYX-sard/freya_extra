use freya::prelude::*;
use freya_extra::dnd_table::{Column, ColumnWidth, DataTable};

use crate::{
    model::{ModRow, Query, apply_enabled, apply_order, build_view},
    state::ModManagerContext,
};

// An alias lets the compiler infer `R` for the closures below.
type ModColumn = Column<ModRow>;

fn enabled_cell(row: &ModRow, context: ModManagerContext) -> Element {
    let (id, mut rows) = (row.id, context.rows);

    rect()
        .height(Size::fill())
        .cross_align(Alignment::Center)
        .child(Checkbox::new().selected(row.enabled).size(16.))
        .on_focus_press(move |_| {
            let mut next = rows.read().clone();

            if let Some(row) = next.iter_mut().find(|row| row.id == id) {
                row.enabled = !row.enabled;
                rows.set(next);
            }
        })
        .into_element()
}

fn columns(context: ModManagerContext) -> Vec<ModColumn> {
    vec![
        ModColumn::new("enabled", "Enabled", move |row| enabled_cell(row, context))
            .width(ColumnWidth::Ratio(0.07))
            .resizable(false)
            .sort_by_key(|row| row.enabled),
        ModColumn::text("id", "ID", |row| row.id.to_string())
            .width(ColumnWidth::Ratio(0.05))
            .sort_by_key(|row| row.id),
        ModColumn::text("name", "Mod Name", |row| row.name.clone())
            .width(ColumnWidth::Ratio(0.27))
            .sort_by_key(|row| row.name.clone()),
        ModColumn::text("type", "Type", |row| row.mod_type.clone())
            .width(ColumnWidth::Ratio(0.12))
            .sort_by_key(|row| row.mod_type.clone()),
        ModColumn::text("url", "URL", |row| row.site.clone())
            .width(ColumnWidth::Ratio(0.27))
            .sort_by_key(|row| row.site.clone()),
        ModColumn::text("priority", "Priority", |row| row.priority.to_string())
            .width(ColumnWidth::Ratio(0.10))
            .sort_by_key(|row| row.priority),
    ]
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
        };

        let view: Vec<ModRow> = build_view(&all, &query).into_iter().cloned().collect();
        let (shown, total) = (view.len(), all.len());

        let rows = context.rows;
        let density = *context.density.read();

        let table = DataTable::new(columns(context), view, |row| row.id)
            .reorderable_rows()
            .reorderable_columns()
            .default_sort("priority", true)
            .on_rows_reorder(move |ids| {
                let mut state = rows;
                apply_order(&mut state.write(), ids);
            })
            .table(move |table| table.density(density).max_height(555.0));

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
                        "Drag a row onto another to change priority, or a header to move its \
                         column. Right-click for bulk actions. ({shown} of {total} shown)"
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
    mut rows: State<Vec<ModRow>>,
    scope: Option<Vec<usize>>,
    enabled: bool,
) -> Box<dyn FnMut()> {
    Box::new(move || {
        apply_enabled(&mut rows.write(), scope.as_deref(), enabled);
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
