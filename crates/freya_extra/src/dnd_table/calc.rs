// Pure, UI-independent calculations for `DndTable`.

use std::{collections::HashSet, hash::Hash};

/// Active sort: `(column_index, ascending)`.
pub type SortSpec = (usize, bool);

/// Direction of the first click on a sortable header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub(super) const fn is_ascending(self) -> bool {
        matches!(self, Self::Ascending)
    }
}

/// How a column is sized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    /// Fixed pixel width. Only fixed columns can be resized by the user.
    Px(f32),
    /// Fraction of the table width (`0.25` = 25%). Stays proportional when the table is resized.
    Ratio(f32),
    /// Take the remaining width of the table. Only the last column can fill.
    Fill,
}

/// Move `items[from]` next to `items[target]`: before it when `before`, after it otherwise.
///
/// Returns `false`, leaving `items` untouched, when an index is out of range or the move
/// would not change the order. Use it to apply the arguments of
/// [`DndTable::on_column_reorder`](super::DndTable::on_column_reorder) to your own column list.
pub fn move_column<T>(items: &mut Vec<T>, from: usize, target: usize, before: bool) -> bool {
    let boundary = target + usize::from(!before);

    if from >= items.len() || target >= items.len() || is_noop_move(from, boundary) {
        return false;
    }

    let item = items.remove(from);
    // Removing the item shifts every boundary behind it by one.
    let index = if from < boundary { boundary - 1 } else { boundary };
    items.insert(index, item);

    true
}

// Dropping a column at the boundary on either of its own sides keeps the order.
const fn is_noop_move(from: usize, boundary: usize) -> bool {
    boundary == from || boundary == from + 1
}

// Keep the entries of `order` that are still `present`, then append the new ones.
pub(super) fn reconcile_order<K: Clone + Eq + Hash>(order: &[K], present: &[K]) -> Vec<K> {
    let present_set: HashSet<&K> = present.iter().collect();
    let mut result: Vec<K> =
        order.iter().filter(|key| present_set.contains(key)).cloned().collect();
    let known: HashSet<&K> = order.iter().collect();

    result.extend(present.iter().filter(|key| !known.contains(key)).cloned());
    result
}

// Keep every entry of `known`, even absent ones (e.g. filtered-out rows keep their place),
// then append the `present` entries that were never seen.
pub(super) fn merge_known_order<K: Clone + Eq + Hash>(known: &[K], present: &[K]) -> Vec<K> {
    let seen: HashSet<&K> = known.iter().collect();
    let mut result = known.to_vec();

    result.extend(present.iter().filter(|key| !seen.contains(key)).cloned());
    result
}

// Freya stores the vertical scroll position as a non-positive value (0 at the top),
// while all geometry math uses a non-negative "scrolled distance".
const SCROLL_Y_SIGN: i32 = -1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ResizeState {
    pub column: usize,
    pub start_x: f64,
    pub start_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DropPosition {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DropTarget {
    pub row_id: usize,
    pub position: DropPosition,
}

// Drag payload of a header cell: the display index of the dragged column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ColumnDrag(pub usize);

// Where the dragged column would land: next to `target`, on its left when `before`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ColumnDropTarget {
    pub from: usize,
    pub target: usize,
    pub before: bool,
}

impl ColumnDropTarget {
    // Insertion boundary in `0..=column_count`, measured before removing the dragged column.
    pub(super) const fn boundary(self) -> usize {
        self.target + if self.before { 0 } else { 1 }
    }
}

// Edge of a header cell that carries the insertion marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColumnMarker {
    Left,
    Right,
}

// Vertical row bounds in global coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RowBounds {
    pub row_id: usize,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HandleState {
    Idle,
    Hovered,
    Active,
}

impl HandleState {
    pub(super) const fn resolve(is_resizing: bool, is_hovered: bool) -> Self {
        if is_resizing {
            Self::Active
        } else if is_hovered {
            Self::Hovered
        } else {
            Self::Idle
        }
    }
}

// --- Scrolling ---------------------------------------------------------------------------------

pub(super) fn scroll_distance(raw_y: i32) -> i32 {
    (raw_y * SCROLL_Y_SIGN).max(0)
}

pub(super) fn raw_scroll_y(distance: i32) -> i32 {
    distance.max(0) * SCROLL_Y_SIGN
}

// One insert line above and below every row.
pub(super) fn content_height(row_count: usize, row_height: f32, line_height: f32) -> f64 {
    (row_count as f64).mul_add(f64::from(row_height + line_height), f64::from(line_height))
}

pub(super) fn max_scroll_y(content_height: f64, viewport_height: f64) -> i32 {
    (content_height - viewport_height).max(0.0) as i32
}

pub(super) fn auto_scroll_y(
    pointer_y: f64,
    viewport_top: f64,
    viewport_bottom: f64,
    current_y: i32,
    max_scroll_y: i32,
    edge: f64,
    step: i32,
) -> i32 {
    if pointer_y < viewport_top + edge {
        current_y.saturating_sub(step).clamp(0, max_scroll_y)
    } else if pointer_y > viewport_bottom - edge {
        current_y.saturating_add(step).clamp(0, max_scroll_y)
    } else {
        current_y.clamp(0, max_scroll_y)
    }
}

// --- Geometry ----------------------------------------------------------------------------------

// `global_y` / `local_y`: pointer position relative to the window / to the row.
pub(super) fn viewport_top(
    global_y: f64,
    local_y: f64,
    row_index: usize,
    scroll_y: i32,
    row_height: f32,
    line_height: f32,
) -> f64 {
    let row_top = global_y - local_y;
    let content_row_top =
        f64::from((row_index as f32).mul_add(row_height + line_height, line_height));

    row_top - content_row_top + f64::from(scroll_y)
}

pub(super) fn row_bounds(
    row_ids: &[usize],
    viewport_top: f64,
    scroll_y: i32,
    row_height: f32,
    line_height: f32,
) -> Vec<RowBounds> {
    let row_height = f64::from(row_height);
    let line_height = f64::from(line_height);
    let stride = row_height + line_height;

    row_ids
        .iter()
        .enumerate()
        .map(|(index, row_id)| {
            let top = viewport_top + line_height + index as f64 * stride - f64::from(scroll_y);

            RowBounds { row_id: *row_id, top, bottom: top + row_height }
        })
        .collect()
}

// --- Drag & drop -------------------------------------------------------------------------------

// Top boundary inclusive, bottom exclusive. `None` if the pointer is outside every row.
pub(super) fn find_drop_target(rows: &[RowBounds], pointer_y: f64) -> Option<DropTarget> {
    let row = rows.iter().find(|row| pointer_y >= row.top && pointer_y < row.bottom)?;

    let midpoint = row.top + (row.bottom - row.top) / 2.0;

    Some(DropTarget {
        row_id: row.row_id,
        position: if pointer_y < midpoint { DropPosition::Before } else { DropPosition::After },
    })
}

// Boundary in `0..=row_ids.len()`, measured before removing the dragged row.
pub(super) fn insertion_boundary(row_ids: &[usize], target: DropTarget) -> Option<usize> {
    let target_index = row_ids.iter().position(|row_id| *row_id == target.row_id)?;

    Some(match target.position {
        DropPosition::Before => target_index,
        DropPosition::After => target_index + 1,
    })
}

pub(super) fn is_insertion_boundary(
    row_ids: &[usize],
    target: Option<DropTarget>,
    boundary: usize,
) -> bool {
    target.is_some_and(|target| insertion_boundary(row_ids, target) == Some(boundary))
}

// `(from_id, target_row_id, before)`; `None` when dropped onto itself.
pub(super) fn reorder_args(from_id: usize, target: DropTarget) -> Option<(usize, usize, bool)> {
    (target.row_id != from_id).then_some((
        from_id,
        target.row_id,
        target.position == DropPosition::Before,
    ))
}

// --- Columns -----------------------------------------------------------------------------------

pub(super) fn resized_width(
    start_width: f32,
    start_x: f64,
    current_x: f64,
    min: f32,
    max: f32,
) -> f32 {
    (start_width + (current_x - start_x) as f32).clamp(min, max)
}

pub(super) fn set_column_width(widths: &mut Vec<Option<f32>>, column: usize, width: f32) {
    if widths.len() <= column {
        widths.resize(column + 1, None);
    }
    widths[column] = Some(width);
}

pub(super) const fn is_last_column(column: usize, column_count: usize) -> bool {
    column == column_count.saturating_sub(1)
}

// User override (a ratio) first, then the declared width. Only the last column may fill.
//
// `Ratio` is resolved to pixels with `table_width`. Until the table has been measured
// (`table_width <= 0.0`) it falls back to the minimum width.
pub(super) fn resolve_column_width(
    override_ratio: Option<f32>,
    declared: Option<ColumnWidth>,
    is_last: bool,
    min_column_width: f32,
    max_column_width: f32,
    table_width: f32,
) -> ColumnWidth {
    let to_px = |ratio: f32| {
        if table_width > 0.0 {
            ColumnWidth::Px((ratio * table_width).clamp(min_column_width, max_column_width))
        } else {
            ColumnWidth::Px(min_column_width)
        }
    };

    // A moved column may carry an override into the last slot, which has to keep filling.
    let keeps_fill = is_last && matches!(declared, Some(ColumnWidth::Fill));

    if let Some(ratio) = override_ratio
        && !keeps_fill
    {
        return to_px(ratio);
    }

    match declared.unwrap_or(ColumnWidth::Px(min_column_width)) {
        ColumnWidth::Fill if !is_last => ColumnWidth::Px(min_column_width),
        ColumnWidth::Ratio(ratio) => to_px(ratio),
        declared => declared,
    }
}

// Pixel width -> fraction of the table, `None` while the table is not measured yet.
pub(super) fn px_to_ratio(px: f32, table_width: f32) -> Option<f32> {
    (table_width > 0.0).then(|| px / table_width)
}

// Resizable columns draw their separator inside the resize handle instead.
pub(super) const fn separator_width(
    show_column_lines: bool,
    is_last: bool,
    resizable: bool,
) -> f32 {
    if show_column_lines && !is_last && !resizable { 1.0 } else { 0.0 }
}

pub(super) fn content_width(width: ColumnWidth, handle_width: f32, separator: f32) -> ColumnWidth {
    match width {
        ColumnWidth::Px(px) => ColumnWidth::Px((px - handle_width - separator).max(0.0)),
        // `Ratio` is always resolved to `Px` before this point.
        ColumnWidth::Ratio(_) | ColumnWidth::Fill => width,
    }
}

// Pixel width of every column. `Fill` takes what the fixed columns leave over.
pub(super) fn pixel_widths(
    widths: &[ColumnWidth],
    table_width: f32,
    min_column_width: f32,
) -> Vec<f32> {
    let fixed: f32 = widths
        .iter()
        .map(|width| match width {
            ColumnWidth::Px(px) => *px,
            ColumnWidth::Ratio(ratio) => ratio * table_width,
            ColumnWidth::Fill => 0.0,
        })
        .sum();
    let fill = (table_width - fixed).max(min_column_width);

    widths
        .iter()
        .map(|width| match width {
            ColumnWidth::Px(px) => *px,
            ColumnWidth::Ratio(ratio) => ratio * table_width,
            ColumnWidth::Fill => fill,
        })
        .collect()
}

// `(left, right)` of every column in global coordinates, from the table's left edge.
pub(super) fn column_bounds(pixel_widths: &[f32], table_left: f64) -> Vec<(f64, f64)> {
    let mut left = table_left;

    pixel_widths
        .iter()
        .map(|width| {
            let right = left + f64::from(*width);
            let bounds = (left, right);
            left = right;
            bounds
        })
        .collect()
}

// A column can be dropped while the pointer is within one header height of the header.
pub(super) fn header_drop_band(table_top: f64, header_height: f32) -> (f64, f64) {
    let height = f64::from(header_height);

    (table_top - height, table_top + height * 2.0)
}

// Left of the first column counts as before it, right of the last one as after it.
// `None` outside the band, or when the drop would not change the order.
pub(super) fn find_column_drop_target(
    bounds: &[(f64, f64)],
    from: usize,
    pointer_x: f64,
    pointer_y: f64,
    band: (f64, f64),
) -> Option<ColumnDropTarget> {
    if pointer_y < band.0 || pointer_y >= band.1 {
        return None;
    }

    let last = bounds.len().checked_sub(1)?;
    let target = bounds.iter().position(|(_, right)| pointer_x < *right).unwrap_or(last);
    let (left, right) = *bounds.get(target)?;
    let before = pointer_x < left + (right - left) / 2.0;

    let drop = ColumnDropTarget { from, target, before };

    (!is_noop_move(from, drop.boundary())).then_some(drop)
}

pub(super) fn column_marker(
    target: Option<ColumnDropTarget>,
    column: usize,
    column_count: usize,
) -> Option<ColumnMarker> {
    let boundary = target?.boundary();

    if boundary == column {
        Some(ColumnMarker::Left)
    } else if boundary == column_count && is_last_column(column, column_count) {
        Some(ColumnMarker::Right)
    } else {
        None
    }
}

// --- Sorting -----------------------------------------------------------------------------------

/// The sort that is in effect: the user's choice, else the table's default order.
///
/// `None` from the user means "reset to the default order", so the default must be
/// rendered with the same arrow as an explicit choice.
pub(super) fn effective_sort(
    current: Option<SortSpec>,
    default: Option<SortSpec>,
) -> Option<SortSpec> {
    current.or(default)
}

/// Direction of the first click on `column`.
///
/// The table default wins for its own column, because that column is already in that
/// order: the first click must flip it, not repeat it.
pub(super) fn first_direction(
    column: usize,
    default: Option<SortSpec>,
    declared_ascending: bool,
) -> bool {
    match default {
        Some((sorted, ascending)) if sorted == column => ascending,
        _ => declared_ascending,
    }
}

// `first` -> opposite -> reset to default (`None`).
pub(super) fn next_sort(
    current: Option<SortSpec>,
    default: Option<SortSpec>,
    column: usize,
    declared_ascending: bool,
) -> Option<SortSpec> {
    let first = first_direction(column, default, declared_ascending);

    match effective_sort(current, default) {
        Some((sorted, ascending)) if sorted == column && ascending == first => {
            Some((column, !first))
        }
        Some((sorted, _)) if sorted == column => None,
        _ => Some((column, first)),
    }
}

pub(super) fn header_text(
    title: &str,
    sort: Option<SortSpec>,
    column: usize,
    sortable: bool,
) -> String {
    match sort {
        Some((sorted, ascending)) if sortable && sorted == column => {
            let arrow = if ascending { "↑" } else { "↓" };
            format!("{title} {arrow}")
        }
        _ => title.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // #[expect(clippy::float_cmp)]
    fn resized_width_clamps_to_limits() {
        assert_eq!(resized_width(100.0, 50.0, 70.0, 48.0, 1200.0), 120.0);
        assert_eq!(resized_width(100.0, 50.0, -500.0, 48.0, 1200.0), 48.0);
        assert_eq!(resized_width(100.0, 50.0, 5000.0, 48.0, 1200.0), 1200.0);
    }

    #[test]
    fn find_drop_target_identifies_row_and_side() {
        let rows = [
            RowBounds { row_id: 10, top: 100.0, bottom: 140.0 },
            RowBounds { row_id: 20, top: 140.0, bottom: 180.0 },
        ];

        assert_eq!(
            find_drop_target(&rows, 110.0),
            Some(DropTarget { row_id: 10, position: DropPosition::Before })
        );
        assert_eq!(
            find_drop_target(&rows, 130.0),
            Some(DropTarget { row_id: 10, position: DropPosition::After })
        );
        assert_eq!(
            find_drop_target(&rows, 140.0),
            Some(DropTarget { row_id: 20, position: DropPosition::Before })
        );
        assert_eq!(find_drop_target(&rows, 180.0), None);
        assert_eq!(find_drop_target(&rows, 90.0), None);
    }

    #[test]
    fn insertion_boundary_resolves_target_row() {
        let row_ids = [10, 20, 30, 40];

        assert_eq!(
            insertion_boundary(&row_ids, DropTarget { row_id: 20, position: DropPosition::Before }),
            Some(1)
        );
        assert_eq!(
            insertion_boundary(&row_ids, DropTarget { row_id: 20, position: DropPosition::After }),
            Some(2)
        );
        assert_eq!(
            insertion_boundary(&row_ids, DropTarget { row_id: 99, position: DropPosition::Before }),
            None
        );
    }

    #[test]
    fn is_insertion_boundary_matches_only_the_resolved_line() {
        let row_ids = [10, 20, 30];
        let target = Some(DropTarget { row_id: 20, position: DropPosition::After });

        assert!(is_insertion_boundary(&row_ids, target, 2));
        assert!(!is_insertion_boundary(&row_ids, target, 1));
        assert!(!is_insertion_boundary(&row_ids, None, 2));
    }

    #[test]
    fn reorder_args_ignores_drop_onto_self() {
        let before = DropTarget { row_id: 20, position: DropPosition::Before };
        let after = DropTarget { row_id: 20, position: DropPosition::After };

        assert_eq!(reorder_args(10, before), Some((10, 20, true)));
        assert_eq!(reorder_args(10, after), Some((10, 20, false)));
        assert_eq!(reorder_args(20, before), None);
    }

    #[test]
    fn auto_scroll_moves_above_and_below_viewport() {
        assert_eq!(auto_scroll_y(-10.0, 0.0, 100.0, 48, 200, 28.0, 24), 24);
        assert_eq!(auto_scroll_y(110.0, 0.0, 100.0, 48, 200, 28.0, 24), 72);
        assert_eq!(auto_scroll_y(50.0, 0.0, 100.0, 48, 200, 28.0, 24), 48);
    }

    #[test]
    fn auto_scroll_stays_within_scroll_limits() {
        assert_eq!(auto_scroll_y(-10.0, 0.0, 100.0, 8, 200, 28.0, 24), 0);
        assert_eq!(auto_scroll_y(110.0, 0.0, 100.0, 192, 200, 28.0, 24), 200);
    }

    #[test]
    fn scroll_conversion_round_trips_and_clamps() {
        assert_eq!(scroll_distance(-48), 48);
        assert_eq!(scroll_distance(5), 0);
        assert_eq!(raw_scroll_y(48), -48);
        assert_eq!(raw_scroll_y(-3), 0);
    }

    #[test]
    fn content_height_and_max_scroll() {
        assert_eq!(content_height(3, 36.0, 4.0), 124.0);
        assert_eq!(max_scroll_y(124.0, 100.0), 24);
        assert_eq!(max_scroll_y(50.0, 100.0), 0);
    }

    #[test]
    fn row_bounds_follow_scroll_offset() {
        let rows = row_bounds(&[1, 2], 100.0, 10, 36.0, 4.0);

        assert_eq!(rows[0], RowBounds { row_id: 1, top: 94.0, bottom: 130.0 });
        assert_eq!(rows[1], RowBounds { row_id: 2, top: 134.0, bottom: 170.0 });
    }

    #[test]
    fn viewport_top_cancels_row_offset_and_scroll() {
        assert_eq!(viewport_top(150.0, 10.0, 2, 8, 36.0, 4.0), 64.0);
    }

    #[test]
    fn set_column_width_grows_the_override_list() {
        let mut widths = vec![None; 1];
        set_column_width(&mut widths, 2, 90.0);

        assert_eq!(widths, vec![None, None, Some(90.0)]);
    }

    #[test]
    fn resolve_column_width_applies_priority_and_fill_rules() {
        use ColumnWidth::{Fill, Px, Ratio};

        let resolve = |o, d, last, table| resolve_column_width(o, d, last, 48.0, 1200.0, table);

        assert_eq!(resolve(Some(0.1), Some(Fill), false, 800.0), Px(80.0));
        assert_eq!(resolve(None, Some(Fill), false, 800.0), Px(48.0));
        assert_eq!(resolve(None, Some(Fill), true, 800.0), Fill);
        assert_eq!(resolve(None, Some(Px(120.0)), false, 800.0), Px(120.0));
        assert_eq!(resolve(None, None, false, 800.0), Px(48.0));

        // Ratio follows the table width and respects the clamp.
        assert_eq!(resolve(None, Some(Ratio(0.25)), false, 800.0), Px(200.0));
        assert_eq!(resolve(None, Some(Ratio(0.25)), false, 1600.0), Px(400.0));
        assert_eq!(resolve(None, Some(Ratio(0.01)), false, 800.0), Px(48.0));
        assert_eq!(resolve(None, Some(Ratio(0.25)), false, 0.0), Px(48.0));
    }

    #[test]
    fn px_to_ratio_needs_a_measured_table() {
        assert_eq!(px_to_ratio(200.0, 800.0), Some(0.25));
        assert_eq!(px_to_ratio(200.0, 0.0), None);
    }

    #[test]
    fn separator_and_content_width() {
        assert_eq!(separator_width(true, false, false), 1.0);
        assert_eq!(separator_width(true, true, false), 0.0);
        assert_eq!(separator_width(true, false, true), 0.0);
        assert_eq!(separator_width(false, false, false), 0.0);

        assert_eq!(content_width(ColumnWidth::Px(100.0), 12.0, 1.0), ColumnWidth::Px(87.0));
        assert_eq!(content_width(ColumnWidth::Px(5.0), 12.0, 0.0), ColumnWidth::Px(0.0));
        assert_eq!(content_width(ColumnWidth::Fill, 12.0, 1.0), ColumnWidth::Fill);
    }

    #[test]
    fn next_sort_cycles_asc_desc_none() {
        assert_eq!(next_sort(None, None, 1, true), Some((1, true)));
        assert_eq!(next_sort(Some((1, true)), None, 1, true), Some((1, false)));
        assert_eq!(next_sort(Some((1, false)), None, 1, true), None);
        assert_eq!(next_sort(Some((2, false)), None, 1, true), Some((1, true)));
    }

    #[test]
    fn next_sort_uses_declared_first_direction() {
        // A column that should start descending (e.g. "modified date").
        assert_eq!(next_sort(None, None, 3, false), Some((3, false)));
        assert_eq!(next_sort(Some((3, false)), None, 3, false), Some((3, true)));
        assert_eq!(next_sort(Some((3, true)), None, 3, false), None);
    }

    #[test]
    fn next_sort_flips_the_default_column_on_first_click() {
        let default = Some((5, true)); // priority ascending is the initial order

        // Unsorted == default order, so the first click goes descending, not ascending again.
        assert_eq!(next_sort(None, default, 5, true), Some((5, false)));
        assert_eq!(next_sort(Some((5, false)), default, 5, true), None);
        // The declared direction is ignored for the default column.
        assert_eq!(next_sort(None, default, 5, false), Some((5, false)));
        // Other columns are unaffected.
        assert_eq!(next_sort(None, default, 2, true), Some((2, true)));
    }

    #[test]
    fn next_sort_handles_a_descending_default() {
        let default = Some((5, false));

        assert_eq!(next_sort(None, default, 5, true), Some((5, true)));
        assert_eq!(next_sort(Some((5, true)), default, 5, true), None);
    }

    #[test]
    fn header_text_shows_arrow_only_for_active_sortable_column() {
        assert_eq!(header_text("Name", Some((0, true)), 0, true), "Name ↑");
        assert_eq!(header_text("Name", Some((0, false)), 0, true), "Name ↓");
        assert_eq!(header_text("Name", Some((0, true)), 0, false), "Name");
        assert_eq!(header_text("Name", Some((1, true)), 0, true), "Name");
        assert_eq!(header_text("Name", None, 0, true), "Name");
    }

    #[test]
    fn handle_state_prefers_active_over_hovered() {
        assert_eq!(HandleState::resolve(true, true), HandleState::Active);
        assert_eq!(HandleState::resolve(false, true), HandleState::Hovered);
        assert_eq!(HandleState::resolve(false, false), HandleState::Idle);
    }

    #[test]
    fn move_column_places_item_before_or_after_target() {
        let moved = |from, target, before| {
            let mut items = vec!['a', 'b', 'c', 'd'];
            move_column(&mut items, from, target, before).then_some(items)
        };

        assert_eq!(moved(0, 2, false), Some(vec!['b', 'c', 'a', 'd']));
        assert_eq!(moved(0, 2, true), Some(vec!['b', 'a', 'c', 'd']));
        assert_eq!(moved(3, 1, true), Some(vec!['a', 'd', 'b', 'c']));
        assert_eq!(moved(3, 1, false), Some(vec!['a', 'b', 'd', 'c']));
        assert_eq!(moved(0, 3, false), Some(vec!['b', 'c', 'd', 'a']));
        assert_eq!(moved(3, 0, true), Some(vec!['d', 'a', 'b', 'c']));
    }

    #[test]
    fn move_column_rejects_noops_and_out_of_range() {
        let mut items = vec!['a', 'b', 'c'];

        assert!(!move_column(&mut items, 1, 1, true));
        assert!(!move_column(&mut items, 1, 1, false));
        assert!(!move_column(&mut items, 1, 0, false));
        assert!(!move_column(&mut items, 1, 2, true));
        assert!(!move_column(&mut items, 3, 0, true));
        assert!(!move_column(&mut items, 0, 3, true));
        assert_eq!(items, vec!['a', 'b', 'c']);
    }

    #[test]
    fn resolve_column_width_keeps_the_last_fill_column_filling() {
        use ColumnWidth::{Fill, Px};

        // A resized column dropped into the last slot must not stop filling.
        assert_eq!(resolve_column_width(Some(0.25), Some(Fill), true, 48.0, 1200.0, 800.0), Fill);
        // A fixed last column still honors the override.
        assert_eq!(
            resolve_column_width(Some(0.25), Some(Px(100.0)), true, 48.0, 1200.0, 800.0),
            Px(200.0)
        );
    }

    #[test]
    fn pixel_widths_give_fill_the_remaining_width() {
        use ColumnWidth::{Fill, Px, Ratio};

        assert_eq!(
            pixel_widths(&[Px(100.0), Ratio(0.25), Fill], 800.0, 48.0),
            vec![100.0, 200.0, 500.0]
        );
        assert_eq!(pixel_widths(&[Px(790.0), Fill], 800.0, 48.0), vec![790.0, 48.0]);
    }

    #[test]
    fn column_bounds_are_contiguous() {
        assert_eq!(
            column_bounds(&[100.0, 50.0, 25.0], 10.0),
            vec![(10.0, 110.0), (110.0, 160.0), (160.0, 185.0)]
        );
    }

    #[test]
    fn find_column_drop_target_picks_column_and_side() {
        let bounds = column_bounds(&[100.0, 100.0, 100.0], 0.0);
        let band = header_drop_band(0.0, 40.0);
        let find = |from, x, y| find_column_drop_target(&bounds, from, x, y, band);

        assert_eq!(
            find(0, 160.0, 10.0),
            Some(ColumnDropTarget { from: 0, target: 1, before: false })
        );
        assert_eq!(
            find(2, 110.0, 10.0),
            Some(ColumnDropTarget { from: 2, target: 1, before: true })
        );
        // Outside the table the nearest end column is used.
        assert_eq!(
            find(2, -50.0, 10.0),
            Some(ColumnDropTarget { from: 2, target: 0, before: true })
        );
        assert_eq!(
            find(0, 900.0, 10.0),
            Some(ColumnDropTarget { from: 0, target: 2, before: false })
        );
        // No-op: next to the dragged column's own slot.
        assert_eq!(find(1, 150.0, 10.0), None);
        assert_eq!(find(1, 120.0, 10.0), None);
        assert_eq!(find(0, 110.0, 10.0), None);
        assert_eq!(find(0, 50.0, 10.0), None);
        // Before the column after the next one is a real move.
        assert_eq!(
            find(0, 210.0, 10.0),
            Some(ColumnDropTarget { from: 0, target: 2, before: true })
        );
        // Vertical band: one header height of slack on each side.
        assert!(find(0, 160.0, -40.0).is_some());
        assert_eq!(find(0, 160.0, -41.0), None);
        assert_eq!(find(0, 160.0, 80.0), None);
        assert!(find(0, 160.0, 79.0).is_some());
        assert_eq!(find_column_drop_target(&[], 0, 0.0, 0.0, band), None);
    }

    #[test]
    fn column_marker_sits_on_the_insertion_boundary() {
        let before_2 = Some(ColumnDropTarget { from: 0, target: 2, before: true });
        let after_1 = Some(ColumnDropTarget { from: 0, target: 1, before: false });
        let after_last = Some(ColumnDropTarget { from: 0, target: 2, before: false });

        assert_eq!(column_marker(before_2, 2, 3), Some(ColumnMarker::Left));
        assert_eq!(column_marker(before_2, 1, 3), None);
        // "After column 1" and "before column 2" are the same boundary.
        assert_eq!(column_marker(after_1, 2, 3), Some(ColumnMarker::Left));
        assert_eq!(column_marker(after_last, 2, 3), Some(ColumnMarker::Right));
        assert_eq!(column_marker(after_last, 1, 3), None);
        assert_eq!(column_marker(None, 0, 3), None);
    }

    #[test]
    fn reconcile_order_drops_missing_and_appends_new() {
        assert_eq!(reconcile_order(&["b", "a", "c"], &["a", "b", "c"]), vec!["b", "a", "c"]);
        assert_eq!(reconcile_order(&["b", "a", "c"], &["a", "d", "b"]), vec!["b", "a", "d"]);
        assert_eq!(reconcile_order::<&str>(&[], &["x", "y"]), vec!["x", "y"]);
    }

    #[test]
    fn merge_known_order_keeps_hidden_entries_in_place() {
        // 2 is currently filtered out but keeps its slot; 9 is new.
        assert_eq!(merge_known_order(&[3, 2, 1], &[1, 3, 9]), vec![3, 2, 1, 9]);
        assert_eq!(merge_known_order(&[], &[5, 4]), vec![5, 4]);
    }
}
