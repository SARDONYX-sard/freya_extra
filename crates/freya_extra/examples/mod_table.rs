//! Run with: `cargo run --example mod_table`
//! or `cargo run -p freya_extra --example mod_table --profile release`
//!
//! A mod priority manager showing `DndTable` with the glass design.

use std::cmp::Ordering;

use freya::prelude::*;
use freya_extra::{
    acrylic,
    dnd_table::{
        ColumnWidth, Density, DndTable, DndTableBody, DndTableCell, DndTableHead, DndTableRow,
        SortSpec,
    },
    dropdown_menu::DropdownMenu,
    glass::{Elevation, Glass, RADIUS_LARGE, RADIUS_MEDIUM, RADIUS_SMALL, use_glass},
    themes,
};

// =================================================================================================
// Model
// =================================================================================================

#[derive(Clone, PartialEq)]
struct ModRow {
    id: usize,

    /// Load priority. The source `Vec` is always kept in ascending priority order.
    /// Values are renormalized to `1..=n` after every move.
    priority: usize,

    enabled: bool,
    name: String,
    mod_type: String,
    site: String,
}

const COLUMN_TITLES: [&str; 6] = ["Enabled", "ID", "Mod Name", "Type", "URL", "Priority"];

fn initial_rows() -> Vec<ModRow> {
    let mut rows = (1..=99)
        .map(|id| {
            let (name, kind) = match id % 3 {
                1 => ("Nemesis Mod", "Nemesis"),
                2 => ("FNIS Mod", "FNIS"),
                _ => ("Nemesis Extension", "NemesisExt"),
            };

            ModRow {
                id,
                priority: id,
                enabled: id % 4 != 0,
                name: format!("Example {name} #{id}"),
                mod_type: kind.to_string(),
                site: format!("https://example.com/mod/{id}"),
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by_key(|row| row.priority);
    normalize_priorities(&mut rows);
    rows
}

/// Describes the active filters and optional sort order.
///
/// The query does not modify the source rows.
#[derive(Clone)]
struct Query {
    /// Search text, already trimmed and lowercased.
    text: String,
    mod_type: Option<String>,
    enabled_only: bool,
    sort: Option<SortSpec>,
}

fn compare_by_column(column: usize, a: &ModRow, b: &ModRow) -> Ordering {
    match column {
        0 => a.enabled.cmp(&b.enabled),
        1 => a.id.cmp(&b.id),
        2 => a.name.cmp(&b.name),
        3 => a.mod_type.cmp(&b.mod_type),
        4 => a.site.cmp(&b.site),
        5 => a.priority.cmp(&b.priority),
        _ => Ordering::Equal,
    }
}

fn matches_query(row: &ModRow, query: &Query) -> bool {
    if query.enabled_only && !row.enabled {
        return false;
    }

    if query.mod_type.as_ref().is_some_and(|kind| row.mod_type != *kind) {
        return false;
    }

    query.text.is_empty()
        || [&row.name, &row.mod_type, &row.site]
            .iter()
            .any(|field| field.to_lowercase().contains(&query.text))
}

/// Filters the source rows and optionally sorts the resulting view.
///
/// The source rows remain in normalized priority order.
fn build_view(rows: &[ModRow], query: &Query) -> Vec<ModRow> {
    let mut view: Vec<ModRow> =
        rows.iter().filter(|row| matches_query(row, query)).cloned().collect();

    if let Some((column, ascending)) = query.sort {
        view.sort_by(|a, b| {
            let ordering = compare_by_column(column, a, b);

            if ascending { ordering } else { ordering.reverse() }
        });
    }

    view
}

/// Renormalizes priorities to `1..=n`.
///
/// The caller must supply rows in their intended priority order.
fn normalize_priorities(rows: &mut [ModRow]) {
    for (index, row) in rows.iter_mut().enumerate() {
        row.priority = index + 1;
    }
}

/// Reorders a row relative to a target row in the filtered display.
///
/// `visible_ids` contains IDs in displayed order.
/// `moved_id` is the dragged row ID.
/// `target_id` is the row under the pointer.
/// `before` determines whether the moved row goes before or after the target.
///
/// Hidden rows are preserved. The source rows remain in priority order,
/// and priorities are renormalized after the move.
///
/// Returns `true` if a move was performed.
fn reorder_visible(
    rows: &mut Vec<ModRow>,
    visible_ids: &[usize],
    moved_id: usize,
    target_id: usize,
    before: bool,
) -> bool {
    if moved_id == target_id
        || !visible_ids.contains(&moved_id)
        || !visible_ids.contains(&target_id)
    {
        return false;
    }

    let Some(moved_index) = rows.iter().position(|row| row.id == moved_id) else {
        return false;
    };

    let Some(target_index) = rows.iter().position(|row| row.id == target_id) else {
        return false;
    };

    let target_priority = rows[target_index].priority as i64;
    let target_key = target_priority * 2;

    // Use a fractional position immediately before or after the target.
    let moved_key = if before { target_key - 1 } else { target_key + 1 };

    let moved = rows.remove(moved_index);

    let mut keyed: Vec<(i64, ModRow)> =
        rows.drain(..).map(|row| (row.priority as i64 * 2, row)).collect();

    keyed.push((moved_key, moved));
    keyed.sort_by_key(|(key, _)| *key);

    rows.extend(keyed.into_iter().map(|(_, row)| row));
    normalize_priorities(rows);

    true
}

/// Enables or disables all rows, or only rows whose IDs occur in `scope`.
fn apply_enabled(rows: &mut [ModRow], scope: Option<&[usize]>, enabled: bool) {
    for row in rows {
        if scope.is_none_or(|ids| ids.contains(&row.id)) {
            row.enabled = enabled;
        }
    }
}

// =================================================================================================
// Context
// =================================================================================================

/// Shared application state.
///
/// This context is provided by the root component and consumed by UI components.
/// The source rows remain the single source of truth for mod ordering and enabled state.
#[derive(Clone, Copy)]
struct ModManagerContext {
    rows: State<Vec<ModRow>>,
    search: State<String>,
    type_filter: State<Option<String>>,
    enabled_only: State<bool>,
    sort: State<Option<SortSpec>>,
    density: State<Density>,
    pressed: State<Option<usize>>,
    menu: State<Option<(f64, f64)>>,
    menu_hover: State<Option<usize>>,
}

/// Initializes the application state and provides it to descendant components.
fn app() -> impl IntoElement {
    use_init_theme(themes::liquid_glass::theme);

    let rows = use_state(initial_rows);
    let search = use_state(String::new);
    let type_filter = use_state(|| None::<String>);
    let enabled_only = use_state(|| false);
    let sort = use_state(|| None::<SortSpec>);
    let density = use_state(Density::default);
    let pressed = use_state(|| None::<usize>);
    let menu = use_state(|| None::<(f64, f64)>);
    let menu_hover = use_state(|| None::<usize>);

    use_provide_context(|| ModManagerContext {
        rows,
        search,
        type_filter,
        enabled_only,
        sort,
        density,
        pressed,
        menu,
        menu_hover,
    });

    App
}

// =================================================================================================
// Small widgets
// =================================================================================================

/// One segment of a segmented control. The active segment is a raised light pill.
fn segment(
    text: impl Into<String>,
    active: bool,
    glass: Glass,
    on_click: impl FnMut() + 'static,
) -> Element {
    let mut on_click = on_click;

    let text = label().text(text.into()).font_size(13.0).color(if active {
        glass.text
    } else {
        glass.text_secondary
    });

    let mut pill = rect()
        .padding(Gaps::new(4.0, 12.0, 4.0, 12.0))
        .corner_radius(CornerRadius::new_all(RADIUS_SMALL - 1.0))
        .on_mouse_up(move |_| on_click());

    if active {
        pill = pill
            .background(glass.selected)
            .shadow(Shadow::new().y(1.0).blur(3.0).color(Color::from_argb(0x50, 0, 0, 0)));
    }

    pill.child(text).into()
}

/// Single-selection control on a recessed glass track.
fn segmented<T: Clone + PartialEq + 'static>(
    options: Vec<(String, T)>,
    selected: &T,
    glass: Glass,
    on_select: impl Fn(T) + Clone + 'static,
) -> Element {
    glass
        .panel(Elevation::Inset, RADIUS_SMALL)
        .horizontal()
        .padding(2.0)
        .spacing(2.0)
        .children(options.into_iter().map(|(title, value)| {
            let on_select = on_select.clone();
            let active = *selected == value;

            segment(title, active, glass, move || {
                on_select(value.clone());
            })
        }))
        .into()
}

/// Checkbox implemented with themed rectangles.
///
/// A checkbox toggles only if its pointer-down and pointer-up belong to the
/// same checkbox. This prevents dropping a row onto a checkbox from toggling it.
fn checkbox(
    key: usize,
    checked: bool,
    pressed: State<Option<usize>>,
    glass: Glass,
    on_toggle: impl FnMut() + 'static,
) -> Element {
    let mut on_toggle = on_toggle;
    let mut pressed_down = pressed;
    let mut pressed_up = pressed;

    let mut boxed = rect()
        .width(Size::px(16.0))
        .height(Size::px(16.0))
        .corner_radius(CornerRadius::new_all(4.0))
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .border(
            Border::new().fill(if checked { glass.accent } else { glass.control_edge }).width(1.0),
        )
        .background(if checked { glass.accent } else { glass.hover })
        .on_pointer_down(move |event: Event<PointerEventData>| {
            event.stop_propagation();
            pressed_down.set(Some(key));
        })
        .on_mouse_up(move |_| {
            let hit = *pressed_up.read() == Some(key);

            if hit {
                pressed_up.set(None);
                on_toggle();
            }
        });

    if checked {
        boxed = boxed.child(label().text("✓").color(glass.on_accent).font_size(11.0));
    }

    boxed.into()
}

fn toolbar_row(title: &str, control: Element, glass: Glass) -> Element {
    rect()
        .horizontal()
        .spacing(8.0)
        .cross_align(Alignment::Center)
        .child(label().text(title.to_string()).font_size(13.0).color(glass.text_secondary))
        .child(control)
        .into()
}

/// Detects a right-click.
///
/// Keep the event API dependency isolated here.
fn is_right_click(event: &Event<PointerEventData>) -> bool {
    event.button() == Some(MouseButton::Right)
}

/// Builds a floating glass context menu.
///
/// The full-window overlay closes the menu when the user presses outside it.
/// The first two entries form a group; a separator follows them when more entries exist.
fn context_menu(
    at: (f64, f64),
    items: Vec<(String, Box<dyn FnMut()>)>,
    menu: State<Option<(f64, f64)>>,
    hover: State<Option<usize>>,
    glass: Glass,
) -> Element {
    let total = items.len();
    let mut entries: Vec<Element> = Vec::with_capacity(total + 1);

    for (index, (title, mut action)) in items.into_iter().enumerate() {
        let hovered = *hover.read() == Some(index);

        let mut enter = hover;
        let mut leave = hover;
        let mut reset = hover;
        let mut close = menu;

        let mut item = rect()
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
            item = item.background(glass.accent);
        }

        entries.push(
            item.child(label().text(title).font_size(13.0).color(if hovered {
                glass.on_accent
            } else {
                glass.text
            }))
            .into(),
        );

        if index == 1 && total > 2 {
            entries.push(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.0))
                    .margin(Gaps::new(4.0, 6.0, 4.0, 6.0))
                    .background(glass.separator)
                    .into(),
            );
        }
    }

    let panel = glass
        .panel(Elevation::Floating, RADIUS_MEDIUM)
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

// =================================================================================================
// Table components
// =================================================================================================

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

fn enabled_cell(
    row: &ModRow,
    rows: State<Vec<ModRow>>,
    pressed: State<Option<usize>>,
    glass: Glass,
) -> Element {
    let id = row.id;

    DndTableCell::new()
        .column(0)
        .child(checkbox(id, row.enabled, pressed, glass, move || {
            let mut state = rows;
            let mut next = state.read().clone();

            if let Some(target) = next.iter_mut().find(|row| row.id == id) {
                target.enabled = !target.enabled;
                state.set(next);
            }
        }))
        .into_element()
}

fn table_body(
    view: &[ModRow],
    rows: State<Vec<ModRow>>,
    pressed: State<Option<usize>>,
    glass: Glass,
) -> DndTableBody {
    view.iter().enumerate().fold(DndTableBody::new(), |body, (index, row)| {
        body.row(
            row.id,
            DndTableRow::new()
                .id(row.id)
                .position(index)
                .children([
                    enabled_cell(row, rows, pressed, glass),
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

// =================================================================================================
// Toolbar component
// =================================================================================================

#[derive(PartialEq)]
struct Toolbar;

impl Component for Toolbar {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let glass = use_glass();

        let query = Query {
            text: context.search.read().trim().to_lowercase(),
            mod_type: context.type_filter.read().clone(),
            enabled_only: *context.enabled_only.read(),
            sort: *context.sort.read(),
        };

        let all = context.rows.read().clone();

        let mut types: Vec<String> = all.iter().map(|row| row.mod_type.clone()).collect();

        types.sort();
        types.dedup();

        let mut type_options: Vec<(String, Option<String>)> = vec![("All".to_string(), None)];

        type_options.extend(types.into_iter().map(|kind| (kind.clone(), Some(kind))));

        let type_filter = context.type_filter;

        let type_select = segmented(type_options, &query.mod_type, glass, move |value| {
            let mut state = type_filter;
            state.set(value);
        });

        let current_density = *context.density.read();
        let density = context.density;

        let density_select = DropdownMenu::new(current_density)
            .item(Density::Compact, "Compact")
            .item(Density::Standard, "Standard")
            .item(Density::Comfortable, "Comfortable")
            .on_change(move |value| {
                let mut state = density;
                state.set(value);
            });

        let enabled_only = context.enabled_only;

        let enabled_only_box = rect()
            .horizontal()
            .spacing(8.0)
            .cross_align(Alignment::Center)
            .child(checkbox(usize::MAX, query.enabled_only, context.pressed, glass, move || {
                let mut state = enabled_only;
                let next = !*state.read();
                state.set(next);
            }))
            .child(label().text("Enabled only").font_size(13.0));

        glass
            .panel(Elevation::Raised, RADIUS_LARGE)
            .padding(14.0)
            .spacing(12.0)
            // Row 1: Theme | Acrylic
            .child(
                rect()
                    .horizontal()
                    .spacing(16.0)
                    .cross_align(Alignment::Center)
                    .child(toolbar_row("Theme", themes::editor::ThemeEditor.into_element(), glass))
                    .child(toolbar_row("Acrylic", toggle_acrylic().into_element(), glass)),
            )
            // Row 2: Search | Enabled only | Type | Density
            .child(
                rect()
                    .horizontal()
                    .spacing(16.0)
                    .cross_align(Alignment::Center)
                    .child(
                        Input::new(context.search)
                            .placeholder("Search mods...")
                            .width(Size::px(280.0)),
                    )
                    .child(enabled_only_box)
                    .child(toolbar_row("Type", type_select, glass))
                    .child(toolbar_row("Density", density_select.into_element(), glass)),
            )
    }
}

fn toggle_acrylic() -> impl IntoElement {
    let mut enabled = use_state(acrylic::is_enabled);
    let mut error_message = use_state(String::new);

    rect()
        .horizontal()
        .spacing(8.0)
        .child(
            Button::new()
                .on_press(move |_| match acrylic::toggle() {
                    Ok(is_enabled) => {
                        enabled.set(is_enabled);
                        error_message.set(String::new());
                    }
                    Err(e) => error_message.set(e.to_owned()),
                })
                .child(if *enabled.read() { "Acrylic: ON" } else { "Acrylic: OFF" }),
        )
        .child(label().color(Color::RED).text(error_message.read().clone()))
}

// =================================================================================================
// Bulk actions component
// =================================================================================================

#[derive(PartialEq)]
struct BulkActions;

impl Component for BulkActions {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let glass = use_glass();

        let all = context.rows.read().clone();

        let query = Query {
            text: context.search.read().trim().to_lowercase(),
            mod_type: context.type_filter.read().clone(),
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
            root =
                root.child(context_menu(at, menu_items, context.menu, context.menu_hover, glass));
        }

        root
    }
}

// =================================================================================================
// Mod table component
// =================================================================================================

#[derive(PartialEq)]
struct ModTable;

impl Component for ModTable {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<ModManagerContext>();
        let glass = use_glass();

        let all = context.rows.read().clone();

        let query = Query {
            text: context.search.read().trim().to_lowercase(),
            mod_type: context.type_filter.read().clone(),
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
                ColumnWidth::Px(88.0),
                ColumnWidth::Px(56.0),
                ColumnWidth::Px(320.0),
                ColumnWidth::Px(140.0),
                ColumnWidth::Px(320.0),
                ColumnWidth::Fill,
            ])
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
            .child(table_body(&view, context.rows, context.pressed, glass));

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
                    .font_size(12.0)
                    .color(glass.text_secondary),
            )
            .child(table_area)
    }
}

// =================================================================================================
// Root component
// =================================================================================================

#[derive(PartialEq)]
struct App;

impl Component for App {
    fn render(&self) -> impl IntoElement {
        rect()
            .expanded()
            .theme_background()
            // .background(Color::TRANSPARENT)
            .shadow(Shadow::new().blur(8.).color((0, 0, 0)).inset())
            .child(
                rect()
                    .theme_color()
                    .expanded()
                    .padding(16.0)
                    .spacing(12.0)
                    .child(
                        label().text("Mod Priority").font_size(20.0).font_weight(FontWeight::BOLD),
                    )
                    .child(Toolbar)
                    .child(ModTable)
                    .child(BulkActions),
            )
    }
}

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_transparency(true)
                .with_background(Color::TRANSPARENT)
                .with_window_handle(|window| {
                    #[cfg(target_os = "windows")]
                    {
                        use freya::winit::{
                            platform::windows::WindowExtWindows as _,
                            raw_window_handle::RawWindowHandle,
                        };

                        let Ok(handle) = (unsafe { window.window_handle_any_thread() }) else {
                            return;
                        };
                        let hwnd = match handle.as_raw() {
                            RawWindowHandle::Win32(handle) => match handle.hwnd.get().try_into() {
                                Ok(addr) => addr,
                                _ => return,
                            },
                            RawWindowHandle::WinRt(handle) => handle.core_window.as_ptr() as usize,
                            _ => return,
                        };

                        acrylic::register_window(hwnd);
                        if let Err(e) = acrylic::enable() {
                            eprintln!("Failed to enable acrylic backdrop: {e}");
                        }
                    }
                })
                .with_title("Mod Priority Table")
                .with_size(1300.0, 850.0),
        ),
    );
}
