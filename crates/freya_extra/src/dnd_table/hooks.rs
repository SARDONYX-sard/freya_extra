// Shared state, hooks and event-driven behavior of the table.

use std::sync::Arc;

use freya::prelude::*;

use super::{
    TableMetrics,
    calc::{
        self, ColumnDrag, ColumnDropTarget, ColumnWidth, DropTarget, ResizeState, SortDirection,
        SortSpec,
    },
    styled::TableColors,
};

// Callback arguments: dragged row ID, target row ID, `true` when inserting before the target.
pub(super) type ReorderHandler = Arc<dyn Fn(usize, usize, bool)>;
pub(super) type SortHandler = Arc<dyn Fn(Option<SortSpec>)>;
// Callback arguments: dragged column index, target column index, `true` when inserting before it.
pub(super) type ColumnReorderHandler = Arc<dyn Fn(usize, usize, bool)>;

// Per-render snapshot shared with descendants.
#[derive(Clone)]
pub(super) struct TableConfig {
    pub colors: TableColors,
    pub metrics: TableMetrics,
    pub columns: Vec<ColumnWidth>,
    pub show_column_lines: bool,
    pub sort: Option<SortSpec>,
    // Order shown while `sort` is `None`; its column flips on the first click.
    pub default_sort: Option<SortSpec>,
    pub on_reorder: Option<ReorderHandler>,
    pub on_sort: Option<SortHandler>,
    pub on_column_reorder: Option<ColumnReorderHandler>,
}

pub(super) fn same_callback<T: ?Sized>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl PartialEq for TableConfig {
    fn eq(&self, other: &Self) -> bool {
        self.colors == other.colors
            && self.metrics == other.metrics
            && self.columns == other.columns
            && self.show_column_lines == other.show_column_lines
            && self.sort == other.sort
            && self.default_sort == other.default_sort
            && same_callback(&self.on_reorder, &other.on_reorder)
            && same_callback(&self.on_sort, &other.on_sort)
            && same_callback(&self.on_column_reorder, &other.on_column_reorder)
    }
}

#[derive(Clone)]
pub(super) struct DndTableContext {
    pub config: State<TableConfig>,
    // User-resized widths as fractions of the table width.
    pub column_width_overrides: State<Vec<Option<f32>>>,
    // Measured outer width of the table in pixels (`0.0` until the first layout).
    pub table_width: State<f32>,
    pub drop_target: State<Option<DropTarget>>,
    // Stable drag IDs in display order.
    pub row_ids: State<Vec<usize>>,
    pub viewport_bounds: State<Option<(f64, f64)>>,
    pub resize_state: State<Option<ResizeState>>,
    pub hovered_boundary: State<Option<usize>>,
    // Global position of the table's top-left corner, taken when a header drag starts.
    pub table_origin: State<(f64, f64)>,
    pub column_drop: State<Option<ColumnDropTarget>>,
    // A header drag started since the last header press; its release must not sort.
    pub column_drag_moved: State<bool>,
    pub scroll_controller: ScrollController,
}

// Call once, from the root component.
pub(super) fn use_dnd_table_context(next_config: TableConfig) -> DndTableContext {
    let drop_target = use_state(|| None::<DropTarget>);
    let row_ids = use_state(Vec::<usize>::new);
    let viewport_bounds = use_state(|| None::<(f64, f64)>);
    let resize_state = use_state(|| None::<ResizeState>);
    let hovered_boundary = use_state(|| None::<usize>);
    let table_width = use_state(|| 0.0_f32);
    let table_origin = use_state(|| (0.0_f64, 0.0_f64));
    let column_drop = use_state(|| None::<ColumnDropTarget>);
    let column_drag_moved = use_state(|| false);
    let column_count = next_config.columns.len();
    let column_width_overrides = use_state(|| vec![None; column_count]);
    let scroll_controller = use_scroll_controller(ScrollConfig::default);

    let mut config = use_state(|| next_config.clone());
    if *config.read() != next_config {
        config.set(next_config);
    }

    let context = DndTableContext {
        config,
        column_width_overrides,
        table_width,
        drop_target,
        row_ids,
        viewport_bounds,
        resize_state,
        hovered_boundary,
        table_origin,
        column_drop,
        column_drag_moved,
        scroll_controller,
    };

    let _provided_context = use_provide_context(|| context.clone());

    context
}

pub(super) fn use_table_context() -> DndTableContext {
    consume_context::<DndTableContext>()
}

pub(super) fn use_dragged_row_id() -> Option<usize> {
    *use_drag::<usize>().read()
}

// Display index of the column whose header is being dragged.
pub(super) fn use_dragged_column() -> Option<usize> {
    (*use_drag::<ColumnDrag>().read()).map(|ColumnDrag(column)| column)
}

// A row drag is in progress and no column resize is.
pub(super) fn use_row_dragging(context: &DndTableContext) -> bool {
    use_dragged_row_id().is_some() && !context.is_resizing()
}

pub(super) fn scroll_offset_y(controller: ScrollController) -> i32 {
    let (_, raw_y): (i32, i32) = controller.into();
    calc::scroll_distance(raw_y)
}

pub(super) fn scroll_to_offset_y(mut controller: ScrollController, offset: i32) {
    controller.scroll_to_y(calc::raw_scroll_y(offset));
}

impl DndTableContext {
    pub(super) fn metrics(&self) -> TableMetrics {
        self.config.read().metrics
    }

    pub(super) fn style(&self) -> (TableMetrics, TableColors) {
        let config = self.config.read();
        (config.metrics, config.colors)
    }

    pub(super) fn is_resizing(&self) -> bool {
        self.resize_state.read().is_some()
    }

    pub(super) fn is_resizing_column(&self, column: usize) -> bool {
        self.resize_state.read().is_some_and(|state| state.column == column)
    }

    pub(super) fn is_hovered_boundary(&self, column: usize) -> bool {
        *self.hovered_boundary.read() == Some(column)
    }

    pub(super) fn table_width(&self) -> f32 {
        *self.table_width.read()
    }

    // Ratio of the table width, set by the last user resize.
    pub(super) fn column_width_override(&self, column: usize) -> Option<f32> {
        self.column_width_overrides.read().get(column).copied().flatten()
    }

    // --- Column resize -----------------------------------------------------------------------

    pub(super) fn begin_resize(&mut self, column: usize, start_x: f64, start_width: f32) {
        self.resize_state.set(Some(ResizeState { column, start_x, start_width }));
    }

    pub(super) fn end_resize(&mut self) {
        self.resize_state.set(None);
    }

    // Returns `false` when no resize is active.
    pub(super) fn update_resize(&mut self, pointer_x: f64) -> bool {
        let Some(resize) = *self.resize_state.read() else {
            return false;
        };
        let metrics = self.metrics();

        let width = calc::resized_width(
            resize.start_width,
            resize.start_x,
            pointer_x,
            metrics.min_column_width,
            metrics.max_column_width,
        );

        // Store a ratio so the column keeps its proportion when the window is resized.
        // Nothing to store until the table has been measured.
        if let Some(ratio) = calc::px_to_ratio(width, self.table_width()) {
            let current = self.column_width_overrides.read().get(resize.column).copied().flatten();

            if current != Some(ratio) {
                let mut widths = self.column_width_overrides.read().clone();
                calc::set_column_width(&mut widths, resize.column, ratio);
                self.column_width_overrides.set(widths);
            }
        }

        true
    }

    pub(super) fn enter_boundary(&mut self, column: usize) {
        self.hovered_boundary.set(Some(column));
    }

    // Keep the hover if it already moved on to another column.
    pub(super) fn leave_boundary(&mut self, column: usize) {
        if self.is_hovered_boundary(column) {
            self.hovered_boundary.set(None);
        }
    }

    // --- Column drag & drop ------------------------------------------------------------------

    // Pixel width of every column as laid out right now.
    fn column_pixel_widths(&self) -> Vec<f32> {
        let (metrics, columns) = {
            let config = self.config.read();
            (config.metrics, config.columns.clone())
        };
        let table_width = self.table_width();
        let count = columns.len();

        let resolved: Vec<ColumnWidth> = (0..count)
            .map(|column| {
                calc::resolve_column_width(
                    self.column_width_override(column),
                    columns.get(column).copied(),
                    calc::is_last_column(column, count),
                    metrics.min_column_width,
                    metrics.max_column_width,
                    table_width,
                )
            })
            .collect();

        calc::pixel_widths(&resolved, table_width, metrics.min_column_width)
    }

    // The pointer went down on the header cell of `column`. `(local_x, local_y)` is the pointer
    // position inside that cell, which is how the table finds its own origin on screen.
    pub(super) fn begin_header_press(
        &mut self,
        column: usize,
        (global_x, global_y): (f64, f64),
        (local_x, local_y): (f64, f64),
    ) {
        self.column_drag_moved.set_if_modified(false);

        let widths = self.column_pixel_widths();
        let cell_offset: f32 = widths.iter().take(column).sum();

        self.table_origin
            .set_if_modified((global_x - local_x - f64::from(cell_offset), global_y - local_y));
    }

    // Recompute the drop target while a header is dragged.
    pub(super) fn update_column_drag(
        &mut self,
        dragged: Option<usize>,
        pointer_x: f64,
        pointer_y: f64,
    ) {
        let Some(from) = dragged else {
            self.column_drop.set_if_modified(None);
            return;
        };

        self.column_drag_moved.set_if_modified(true);

        let (table_left, table_top) = *self.table_origin.read();
        let header_height = self.metrics().header_height;
        let bounds = calc::column_bounds(&self.column_pixel_widths(), table_left);
        let band = calc::header_drop_band(table_top, header_height);

        self.column_drop.set_if_modified(calc::find_column_drop_target(
            &bounds, from, pointer_x, pointer_y, band,
        ));
    }

    // Apply the last drop target. Called on every pointer release; does nothing without one.
    pub(super) fn finish_column_drag(&mut self) {
        let target = *self.column_drop.read();

        let Some(target) = target else {
            return;
        };

        self.column_drop.set(None);

        let (callback, column_count) = {
            let config = self.config.read();
            (config.on_column_reorder.clone(), config.columns.len())
        };

        let Some(callback) = callback else {
            return;
        };

        // The resized widths belong to the columns, so they travel with them.
        let mut widths = self.column_width_overrides.read().clone();
        widths.resize(widths.len().max(column_count), None);

        if calc::move_column(&mut widths, target.from, target.target, target.before) {
            self.column_width_overrides.set(widths);
        }

        callback(target.from, target.target, target.before);
    }

    // --- Row drag & drop ---------------------------------------------------------------------

    pub(super) fn sync_row_ids(&mut self, ids: &[usize]) {
        if self.row_ids.read().as_slice() != ids {
            self.row_ids.set(ids.to_vec());
        }
    }

    pub(super) fn track_viewport(&mut self, row_index: usize, global_y: f64, local_y: f64) {
        let metrics = self.metrics();
        let scroll_y = scroll_offset_y(self.scroll_controller);

        let viewport_top = calc::viewport_top(
            global_y,
            local_y,
            row_index,
            scroll_y,
            metrics.row_height,
            metrics.insert_line_height,
        );

        self.viewport_bounds
            .set(Some((viewport_top, viewport_top + f64::from(metrics.max_height))));
    }

    // Auto-scroll near the viewport edges, then recompute the drop target.
    pub(super) fn update_drag(&mut self, pointer_y: f64) {
        let metrics = self.metrics();

        let Some((viewport_top, viewport_bottom)) = *self.viewport_bounds.read() else {
            return;
        };

        let controller = self.scroll_controller;
        let current_y = scroll_offset_y(controller);
        let ids = self.row_ids.read().clone();

        let content_height =
            calc::content_height(ids.len(), metrics.row_height, metrics.insert_line_height);
        let max_scroll_y = calc::max_scroll_y(content_height, viewport_bottom - viewport_top);

        let next_y = calc::auto_scroll_y(
            pointer_y,
            viewport_top,
            viewport_bottom,
            current_y,
            max_scroll_y,
            metrics.auto_scroll_edge,
            metrics.auto_scroll_step,
        );

        if next_y != current_y {
            scroll_to_offset_y(controller, next_y);
        }

        // Use the scroll offset expected after this pointer movement.
        let rows = calc::row_bounds(
            &ids,
            viewport_top,
            next_y,
            metrics.row_height,
            metrics.insert_line_height,
        );

        self.drop_target.set(calc::find_drop_target(&rows, pointer_y));
    }

    pub(super) fn finish_drop(&mut self, from_id: usize) {
        if self.is_resizing() {
            self.drop_target.set(None);
            return;
        }

        let target = *self.drop_target.read();
        let callback = self.config.read().on_reorder.clone();

        if let (Some(target), Some(callback)) = (target, callback)
            && let Some((from, to, before)) = calc::reorder_args(from_id, target)
        {
            callback(from, to, before);
        }

        self.drop_target.set(None);
    }

    // --- Sorting -----------------------------------------------------------------------------

    // `first_direction`: direction of the first click, declared by the header cell.
    pub(super) fn request_sort(&self, column: usize, first_direction: SortDirection) {
        if self.is_resizing() || *self.column_drag_moved.read() {
            return;
        }

        let (current, default, callback) = {
            let config = self.config.read();
            (config.sort, config.default_sort, config.on_sort.clone())
        };

        if let Some(callback) = callback {
            let next = calc::next_sort(current, default, column, first_direction.is_ascending());
            callback(next);
        }
    }
}
