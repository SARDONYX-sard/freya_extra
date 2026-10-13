// Shared state, hooks and event-driven behavior of the table.

use std::sync::Arc;

use freya::prelude::*;

use super::{
    TableMetrics,
    calc::{self, ColumnWidth, DropTarget, ResizeState, SortSpec},
    styled::TableColors,
};

// Callback arguments: dragged row ID, target row ID, `true` when inserting before the target.
pub(super) type ReorderHandler = Arc<dyn Fn(usize, usize, bool)>;
pub(super) type SortHandler = Arc<dyn Fn(Option<SortSpec>)>;

// Per-render snapshot shared with descendants.
#[derive(Clone)]
pub(super) struct TableConfig {
    pub colors: TableColors,
    pub metrics: TableMetrics,
    pub columns: Vec<ColumnWidth>,
    pub show_column_lines: bool,
    pub sort: Option<SortSpec>,
    pub on_reorder: Option<ReorderHandler>,
    pub on_sort: Option<SortHandler>,
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
            && same_callback(&self.on_reorder, &other.on_reorder)
            && same_callback(&self.on_sort, &other.on_sort)
    }
}

#[derive(Clone)]
pub(super) struct DndTableContext {
    pub config: State<TableConfig>,
    pub column_width_overrides: State<Vec<Option<f32>>>,
    pub drop_target: State<Option<DropTarget>>,
    // Stable drag IDs in display order.
    pub row_ids: State<Vec<usize>>,
    pub viewport_bounds: State<Option<(f64, f64)>>,
    pub resize_state: State<Option<ResizeState>>,
    pub hovered_boundary: State<Option<usize>>,
    pub scroll_controller: ScrollController,
}

// Call once, from the root component.
pub(super) fn use_dnd_table_context(next_config: TableConfig) -> DndTableContext {
    let drop_target = use_state(|| None::<DropTarget>);
    let row_ids = use_state(Vec::<usize>::new);
    let viewport_bounds = use_state(|| None::<(f64, f64)>);
    let resize_state = use_state(|| None::<ResizeState>);
    let hovered_boundary = use_state(|| None::<usize>);
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
        drop_target,
        row_ids,
        viewport_bounds,
        resize_state,
        hovered_boundary,
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

        let mut widths = self.column_width_overrides.read().clone();
        calc::set_column_width(&mut widths, resize.column, width);
        self.column_width_overrides.set(widths);

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

    pub(super) fn request_sort(&self, column: usize) {
        if self.is_resizing() {
            return;
        }

        let (current, callback) = {
            let config = self.config.read();
            (config.sort, config.on_sort.clone())
        };

        if let Some(callback) = callback {
            callback(calc::next_sort(current, column));
        }
    }
}
