//! A table with drag-and-drop row reordering, resizable columns and sortable headers.
//!
//! See `examples/dnd_table.rs` for a complete usage example.

#![allow(missing_debug_implementations, clippy::too_many_arguments, clippy::missing_const_for_fn)]

use std::sync::Arc;

use freya::prelude::*;

pub use super::calc::{ColumnWidth, SortSpec};
use super::{
    calc::{self, HandleState, is_last_column},
    hooks::{
        DndTableContext, ReorderHandler, SortHandler, TableConfig, same_callback,
        use_dnd_table_context, use_dragged_row_id, use_row_dragging, use_table_context,
    },
    styled::{self, use_table_colors},
};

fn size(width: ColumnWidth) -> Size {
    match width {
        ColumnWidth::Px(px) => Size::px(px),
        ColumnWidth::Fill => Size::fill(),
    }
}

// ---------------------------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------------------------

/// Row density, similar to MUI's `density` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

impl Density {
    const fn row_height(self) -> f32 {
        match self {
            Self::Compact => 28.0,
            Self::Standard => 36.0,
            Self::Comfortable => 48.0,
        }
    }

    const fn header_height(self) -> f32 {
        match self {
            Self::Compact => 32.0,
            Self::Standard => 40.0,
            Self::Comfortable => 52.0,
        }
    }

    const fn cell_padding(self) -> f32 {
        match self {
            Self::Compact => 4.0,
            Self::Standard => 8.0,
            Self::Comfortable => 12.0,
        }
    }

    const fn insert_line_height(self) -> f32 {
        match self {
            Self::Compact => 3.0,
            Self::Standard | Self::Comfortable => 4.0,
        }
    }
}

/// All numeric tuning values of the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableMetrics {
    pub row_height: f32,
    pub header_height: f32,
    pub insert_line_height: f32,
    pub cell_padding: f32,
    pub max_height: f32,
    pub resize_handle_width: f32,
    pub resize_line_width: f32,
    pub resize_line_hover_width: f32,
    pub min_column_width: f32,
    pub max_column_width: f32,
    pub auto_scroll_edge: f64,
    pub auto_scroll_step: i32,
}

impl TableMetrics {
    /// Metrics for a density preset, with defaults for everything else.
    #[inline]
    pub const fn for_density(density: Density) -> Self {
        Self {
            row_height: density.row_height(),
            header_height: density.header_height(),
            insert_line_height: density.insert_line_height(),
            cell_padding: density.cell_padding(),
            max_height: 440.0,
            resize_handle_width: 12.0,
            resize_line_width: 1.0,
            resize_line_hover_width: 3.0,
            min_column_width: 48.0,
            max_column_width: 1200.0,
            auto_scroll_edge: 28.0,
            auto_scroll_step: 24,
        }
    }
}

impl Default for TableMetrics {
    #[inline]
    fn default() -> Self {
        Self::for_density(Density::Standard)
    }
}

// ---------------------------------------------------------------------------------------------
// DndTable
// ---------------------------------------------------------------------------------------------

/// Root component. Compose it from [`DndTableHead`] and [`DndTableBody`].
#[derive(Clone)]
pub struct DndTable {
    children: Vec<Element>,
    columns: Vec<ColumnWidth>,
    metrics: TableMetrics,
    fixed_width: Option<f32>,
    show_column_lines: bool,
    sort: Option<SortSpec>,
    on_reorder: Option<ReorderHandler>,
    on_sort: Option<SortHandler>,
}

impl Default for DndTable {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl DndTable {
    /// Create an empty table with standard density.
    #[inline]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            columns: Vec::new(),
            metrics: TableMetrics::default(),
            fixed_width: None,
            show_column_lines: true,
            sort: None,
            on_reorder: None,
            on_sort: None,
        }
    }

    /// Column sizing, in order. Missing entries default to `min_column_width`.
    #[inline]
    pub fn columns(mut self, columns: impl IntoIterator<Item = ColumnWidth>) -> Self {
        self.columns = columns.into_iter().collect();
        self
    }

    /// Fix the outer width of the table. By default the table fills its parent.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.fixed_width = Some(width);
        self
    }

    /// Apply a density preset before individual metric overrides.
    #[inline]
    pub fn density(mut self, density: Density) -> Self {
        let preset = TableMetrics::for_density(density);
        self.metrics.row_height = preset.row_height;
        self.metrics.header_height = preset.header_height;
        self.metrics.insert_line_height = preset.insert_line_height;
        self.metrics.cell_padding = preset.cell_padding;
        self
    }

    /// Replace all metrics at once.
    #[inline]
    pub fn metrics(mut self, metrics: TableMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Height of a body row in pixels (at least `1.0`).
    #[inline]
    pub fn row_height(mut self, height: f32) -> Self {
        self.metrics.row_height = height.max(1.0);
        self
    }

    /// Height of the header row in pixels (at least `1.0`).
    #[inline]
    pub fn header_height(mut self, height: f32) -> Self {
        self.metrics.header_height = height.max(1.0);
        self
    }

    /// Height of the insertion marker between rows in pixels.
    #[inline]
    pub fn insert_line_height(mut self, height: f32) -> Self {
        self.metrics.insert_line_height = height.max(0.0);
        self
    }

    /// Padding inside every cell in pixels.
    #[inline]
    pub fn cell_padding(mut self, padding: f32) -> Self {
        self.metrics.cell_padding = padding.max(0.0);
        self
    }

    /// Maximum height of the scrollable body in pixels (at least `1.0`).
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        self.metrics.max_height = height.max(1.0);
        self
    }

    /// Lower bound for user-resized columns, also the default column width.
    #[inline]
    pub fn min_column_width(mut self, width: f32) -> Self {
        self.metrics.min_column_width = width.max(1.0);
        self
    }

    /// Upper bound for user-resized columns.
    #[inline]
    pub fn max_column_width(mut self, width: f32) -> Self {
        self.metrics.max_column_width = width.max(1.0);
        self
    }

    /// Width of the draggable resize handle at each column boundary.
    #[inline]
    pub fn resize_handle_width(mut self, width: f32) -> Self {
        self.metrics.resize_handle_width = width.max(1.0);
        self
    }

    /// Auto-scroll while dragging: `edge` is the trigger distance from the viewport
    /// edge, `step` is the scroll distance per pointer move.
    #[inline]
    pub fn auto_scroll(mut self, edge: f64, step: i32) -> Self {
        self.metrics.auto_scroll_edge = edge.max(0.0);
        self.metrics.auto_scroll_step = step.max(1);
        self
    }

    /// Show or hide the vertical lines between columns.
    #[inline]
    pub fn column_lines(mut self, enabled: bool) -> Self {
        self.show_column_lines = enabled;
        self
    }

    /// Active sort, used to render the direction arrow in headers.
    #[inline]
    pub fn sort(mut self, sort: Option<SortSpec>) -> Self {
        self.sort = sort;
        self
    }

    /// Set the reorder callback.
    ///
    /// Arguments are `(from_id, target_row_id, before)`.
    /// The IDs are stable row IDs, not display indices.
    #[inline]
    pub fn on_reorder(mut self, callback: impl Fn(usize, usize, bool) + 'static) -> Self {
        self.on_reorder = Some(Arc::new(callback));
        self
    }

    /// Called when a sortable header is clicked.
    ///
    /// The argument is the next sort: ascending, then descending, then `None`.
    #[inline]
    pub fn on_sort(mut self, callback: impl Fn(Option<SortSpec>) + 'static) -> Self {
        self.on_sort = Some(Arc::new(callback));
        self
    }

    /// Append a child (usually a [`DndTableHead`] or a [`DndTableBody`]).
    #[inline]
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Append several children.
    #[inline]
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }
}

impl Component for DndTable {
    fn render(&self) -> impl IntoElement {
        let context = use_dnd_table_context(TableConfig {
            colors: use_table_colors(),
            metrics: self.metrics,
            columns: self.columns.clone(),
            show_column_lines: self.show_column_lines,
            sort: self.sort,
            on_reorder: self.on_reorder.clone(),
            on_sort: self.on_sort.clone(),
        });
        let dragging = use_row_dragging(&context);

        let outer_width = match self.fixed_width {
            Some(width) => Size::px(width),
            None => Size::fill(),
        };

        let mut move_context = context.clone();
        let on_global_pointer_move = move |event: Event<PointerEventData>| {
            let (pointer_x, pointer_y) = event.global_location().to_tuple();

            if move_context.update_resize(pointer_x) {
                return;
            }

            if dragging {
                move_context.update_drag(pointer_y);
            }
        };

        let mut up_context = context.clone();
        let on_global_pointer_up = move |_| up_context.end_resize();

        let mut drop_context = context.clone();
        let drop_zone = DropZone::new(move |from_id: usize| drop_context.finish_drop(from_id))
            .child(rect().width(outer_width.clone()).spacing(0.0).children(self.children.clone()));

        let mut root = rect()
            .width(outer_width)
            .spacing(0.0)
            .corner_radius(CornerRadius::new(12.0, 12.0, 12.0, 12.0))
            .on_global_pointer_move(on_global_pointer_move)
            .on_global_pointer_up(on_global_pointer_up);

        // Keep the resize cursor over the whole table while resizing, not only over the handle.
        if context.is_resizing() {
            root = root.cursor(CursorIcon::EwResize);
        }

        root.child(drop_zone)
    }
}

impl PartialEq for DndTable {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
            && self.columns == other.columns
            && self.metrics == other.metrics
            && self.fixed_width == other.fixed_width
            && self.show_column_lines == other.show_column_lines
            && self.sort == other.sort
            && same_callback(&self.on_reorder, &other.on_reorder)
            && same_callback(&self.on_sort, &other.on_sort)
    }
}

// ---------------------------------------------------------------------------------------------
// Head
// ---------------------------------------------------------------------------------------------

/// Header container. Put header rows (`DndTableRow::header(true)`) inside.
#[derive(Clone, Default, PartialEq)]
pub struct DndTableHead {
    children: Vec<Element>,
}

impl DndTableHead {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a header row.
    #[inline]
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Append several header rows.
    #[inline]
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }
}

impl Component for DndTableHead {
    fn render(&self) -> impl IntoElement {
        let (metrics, _colors) = use_table_context().style();

        rect()
            .width(Size::fill())
            .height(Size::px(metrics.header_height))
            .spacing(0.0)
            .corner_radius(CornerRadius::new(12.0, 12.0, 0.0, 0.0))
            .background(Color::TRANSPARENT)
            .children(self.children.clone())
    }
}

// ---------------------------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------------------------

/// Scrollable body. Every row is registered together with its stable ID.
#[derive(Clone, Default, PartialEq)]
pub struct DndTableBody {
    rows: Vec<(usize, Element)>,
}

impl DndTableBody {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a row together with its stable ID.
    ///
    /// The ID must match `DndTableRow::id`.
    #[inline]
    pub fn row(mut self, id: usize, row: impl Into<Element>) -> Self {
        self.rows.push((id, row.into()));
        self
    }
}

impl Component for DndTableBody {
    fn render(&self) -> impl IntoElement {
        let mut context = use_table_context();
        let (metrics, colors) = context.style();
        let count = self.rows.len();

        let ids: Vec<usize> = self.rows.iter().map(|(id, _)| *id).collect();
        context.sync_row_ids(&ids);

        let drop_target = context.drop_target;
        let line_height = metrics.insert_line_height;
        let dragging = use_row_dragging(&context);

        let insert_line = move |index: usize| -> Element {
            let target = *drop_target.read();
            let active = dragging && calc::is_insertion_boundary(&ids, target, index);

            styled::insert_line(line_height, active, colors)
        };

        let mut content: Vec<Element> = Vec::with_capacity(count * 2 + 1);

        for (index, (_, row)) in self.rows.iter().cloned().enumerate() {
            content.push(insert_line(index));
            content.push(row);
        }

        content.push(insert_line(count));

        ScrollView::new_controlled(context.scroll_controller)
            .width(Size::fill())
            .height(Size::px(metrics.max_height))
            .max_height(Size::px(metrics.max_height))
            .spacing(0.0)
            .child(rect().width(Size::fill()).spacing(0.0).children(content))
    }
}

// ---------------------------------------------------------------------------------------------
// Row
// ---------------------------------------------------------------------------------------------

/// A table row: plain for headers, draggable and animated for body rows.
#[derive(Clone, Default, PartialEq)]
pub struct DndTableRow {
    children: Vec<Element>,
    row_id: Option<usize>,
    position: Option<usize>,
    is_header: bool,
}

impl DndTableRow {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a cell.
    #[inline]
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Append several cells.
    #[inline]
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }

    /// Make the row draggable and assign its stable ID.
    #[inline]
    pub fn id(mut self, id: usize) -> Self {
        self.row_id = Some(id);
        self
    }

    /// Set the row's current display index. This controls striping and animation.
    #[inline]
    pub fn position(mut self, position: usize) -> Self {
        self.position = Some(position);
        self
    }

    /// Mark the row as a header row (never draggable, uses the header height).
    #[inline]
    pub fn header(mut self, enabled: bool) -> Self {
        self.is_header = enabled;
        self
    }
}

impl Component for DndTableRow {
    fn render(&self) -> impl IntoElement {
        let context = use_table_context();
        let (metrics, colors) = context.style();
        let dragged_id = use_dragged_row_id();

        let is_header = self.is_header;
        let dragging = dragged_id.is_some() && !context.is_resizing();
        let row_height = if is_header { metrics.header_height } else { metrics.row_height };
        let row_width = Size::fill();
        let background = styled::row_background(is_header, self.position, colors);

        // The same content is rendered in the table and in the drag ghost.
        let content = {
            let children = self.children.clone();
            let row_width = row_width.clone();

            move || {
                rect()
                    .horizontal()
                    .width(row_width.clone())
                    .height(Size::px(row_height))
                    .spacing(0.0)
                    .background(background)
                    .children(children.clone())
            }
        };

        let Some(id) = self.row_id.filter(|_| !is_header) else {
            return Element::from(content());
        };

        let portal = || styled::row_portal(id, row_width.clone(), row_height);

        let mut body = rect()
            .width(row_width.clone())
            .height(Size::px(row_height))
            .opacity(styled::row_opacity(dragged_id == Some(id)));

        if let Some(row_index) = self.position {
            let mut tracker = context.clone();

            // Track the viewport geometry. The drop target itself is computed centrally
            // from the current pointer position.
            body = body.on_pointer_move(move |event: Event<PointerEventData>| {
                if dragging {
                    return;
                }

                let (_, global_y) = event.global_location().to_tuple();
                let (_, local_y) = event.element_location().to_tuple();

                tracker.track_viewport(row_index, global_y, local_y);
            });
        }

        let body = body.child(content());
        let drag_element =
            portal().child(styled::drag_ghost(row_width.clone(), row_height, colors, content()));

        DragZone::new(id)
            .enabled(!context.is_resizing())
            .drag_element(drag_element)
            .show_while_dragging(true)
            .child(
                portal().animation_dependency((self.position.unwrap_or(0), dragging)).child(body),
            )
            .key(id)
            .into()
    }
}

// ---------------------------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------------------------

/// A table cell: plain body cell, or header cell (sortable / resizable).
#[derive(Clone, Default, PartialEq)]
pub struct DndTableCell {
    children: Vec<Element>,
    column: Option<usize>,
    title: Option<String>,
    is_header: bool,
    sortable: bool,
    resizable: bool,
}

impl DndTableCell {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append cell content.
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

    /// Index of the column this cell belongs to (defaults to `0`).
    #[inline]
    pub fn column(mut self, index: usize) -> Self {
        self.column = Some(index);
        self
    }

    /// Render the cell as a header cell.
    #[inline]
    pub fn header(mut self, enabled: bool) -> Self {
        self.is_header = enabled;
        self
    }

    /// Header title; shows a direction arrow when this column is the active sort.
    #[inline]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Make the header clickable to cycle the sort (ascending, descending, none).
    #[inline]
    pub fn sortable(mut self, enabled: bool) -> Self {
        self.sortable = enabled;
        self
    }

    /// Allow the user to resize the column. Only fixed-width columns can be resized.
    #[inline]
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }
}

impl Component for DndTableCell {
    fn render(&self) -> impl IntoElement {
        let context = use_table_context();
        let config = context.config.read().clone();
        let (metrics, colors) = (config.metrics, config.colors);

        let column = self.column.unwrap_or(0);
        let row_dragging = use_dragged_row_id().is_some();
        let is_last = is_last_column(column, config.columns.len());

        let width = calc::resolve_column_width(
            context.column_width_override(column),
            config.columns.get(column).copied(),
            is_last,
            metrics.min_column_width,
        );

        let resizable = self.resizable && matches!(width, ColumnWidth::Px(_));
        let handle_width = if resizable { metrics.resize_handle_width } else { 0.0 };
        let separator_width = calc::separator_width(config.show_column_lines, is_last, resizable);
        let content_width = calc::content_width(width, handle_width, separator_width);

        let mut content = rect()
            .width(size(content_width))
            .height(Size::fill())
            .padding(metrics.cell_padding)
            .spacing(0.0);

        if self.is_header {
            content = content.background(Color::TRANSPARENT);

            if let Some(title) = &self.title {
                let text = calc::header_text(title, config.sort, column, self.sortable);
                content = content.child(styled::header_label(text, colors));
            }
        }

        if self.is_header && self.sortable {
            let sort_context = context.clone();
            content = content.on_mouse_up(move |_| sort_context.request_sort(column));
        }

        let mut parts: Vec<Element> = vec![content.children(self.children.clone()).into()];

        if resizable && let ColumnWidth::Px(px) = width {
            parts.push(resize_handle(&context, metrics, colors, column, px, row_dragging));
        } else if config.show_column_lines && !is_last {
            parts.push(styled::grid_separator(colors));
        }

        rect().horizontal().width(size(width)).height(Size::fill()).spacing(0.0).children(parts)
    }
}

// `width` is the column's current pixel width, used as the start width of a resize.
fn resize_handle(
    context: &DndTableContext,
    metrics: TableMetrics,
    colors: styled::TableColors,
    column: usize,
    width: f32,
    row_dragging: bool,
) -> Element {
    let is_hovered = !row_dragging && context.is_hovered_boundary(column);
    let state = HandleState::resolve(context.is_resizing_column(column), is_hovered);

    let mut enter_context = context.clone();
    let mut leave_context = context.clone();
    let mut down_context = context.clone();

    rect()
        .cursor(CursorIcon::EwResize)
        .width(Size::px(metrics.resize_handle_width))
        .height(Size::fill())
        .horizontal()
        .cross_align(Alignment::Center)
        .on_pointer_enter(move |_| {
            if !row_dragging {
                enter_context.enter_boundary(column);
            }
        })
        .on_pointer_leave(move |_| leave_context.leave_boundary(column))
        .on_pointer_down(move |event: Event<PointerEventData>| {
            if row_dragging {
                return;
            }

            event.stop_propagation();

            let (x, _) = event.global_location().to_tuple();
            down_context.begin_resize(column, x, width);
        })
        .child(styled::resize_line(state, metrics, colors))
        .into()
}
