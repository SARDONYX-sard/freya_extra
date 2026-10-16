//! A data-driven layer over [`DndTable`]: declare columns and rows, and get sorting, row
//! dragging and column dragging with their state kept inside the component.
//!
//! ```ignore
//! DataTable::new(
//!     [
//!         Column::text("name", "Name", |m: &Mod| m.name.clone()).sort_by_key(|m| m.name.clone()),
//!         Column::text("size", "Size", |m: &Mod| m.size.to_string()).sort_by_key(|m| m.size),
//!     ],
//!     mods,
//!     |m| m.id,
//! )
//! .reorderable_rows()
//! .reorderable_columns()
//! ```
//!
//! The state is uncontrolled: the table remembers the order of rows and columns, and the
//! sort, by itself. Use the `on_*_reorder` callbacks to be told about changes. For anything
//! this layer does not cover, build the table from [`DndTable`] directly.

use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use freya::prelude::*;

use super::{
    ColumnWidth, DndTable, DndTableBody, DndTableCell, DndTableHead, DndTableRow, SortDirection,
    SortSpec, calc, hooks::same_callback,
};

type Compare<R> = Arc<dyn Fn(&R, &R) -> Ordering>;
type Render<R> = Arc<dyn Fn(&R) -> Element>;

/// One column of a [`DataTable`].
///
/// A column is identified by its `key`, not by its position, so it can be moved freely.
pub struct Column<R> {
    key: &'static str,
    title: String,
    width: ColumnWidth,
    resizable: bool,
    compare: Option<Compare<R>>,
    first_direction: SortDirection,
    render: Render<R>,
}

impl<R> Clone for Column<R> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            title: self.title.clone(),
            width: self.width,
            resizable: self.resizable,
            compare: self.compare.clone(),
            first_direction: self.first_direction,
            render: Arc::clone(&self.render),
        }
    }
}

impl<R> PartialEq for Column<R> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.title == other.title
            && self.width == other.width
            && self.resizable == other.resizable
            && self.first_direction == other.first_direction
            && same_callback(&self.compare, &other.compare)
            && Arc::ptr_eq(&self.render, &other.render)
    }
}

impl<R: 'static> Column<R> {
    /// A column whose cells are rendered by `render`.
    ///
    /// Defaults: `ColumnWidth::Px(140.0)`, resizable, not sortable.
    pub fn new(
        key: &'static str,
        title: impl Into<String>,
        render: impl Fn(&R) -> Element + 'static,
    ) -> Self {
        Self {
            key,
            title: title.into(),
            width: ColumnWidth::Px(140.0),
            resizable: true,
            compare: None,
            first_direction: SortDirection::default(),
            render: Arc::new(render),
        }
    }

    /// A column of single-line text.
    pub fn text(
        key: &'static str,
        title: impl Into<String>,
        text: impl Fn(&R) -> String + 'static,
    ) -> Self {
        Self::new(key, title, move |row| {
            label()
                .text(text(row))
                .font_size(13.0)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis)
                .into_element()
        })
    }

    /// Width while the column is not the last one. The last column always fills.
    #[inline]
    pub fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }

    /// Allow the user to resize the column.
    #[inline]
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }

    /// Make the header sortable with a custom comparison.
    #[inline]
    pub fn sort_by(mut self, compare: impl Fn(&R, &R) -> Ordering + 'static) -> Self {
        self.compare = Some(Arc::new(compare));
        self
    }

    /// Make the header sortable by a key extracted from each row.
    #[inline]
    pub fn sort_by_key<K: Ord>(self, key: impl Fn(&R) -> K + 'static) -> Self {
        self.sort_by(move |a, b| key(a).cmp(&key(b)))
    }

    /// Direction of the first click on the header (default: ascending).
    #[inline]
    pub fn first_sort_direction(mut self, direction: SortDirection) -> Self {
        self.first_direction = direction;
        self
    }
}

type RowsHandler = Arc<dyn Fn(&[usize])>;
type ColumnsHandler = Arc<dyn Fn(&[&'static str])>;
type ConfigureHandler = Arc<dyn Fn(DndTable) -> DndTable>;

/// A table built from [`Column`]s and rows. See the [module docs](self).
pub struct DataTable<R> {
    columns: Vec<Column<R>>,
    rows: Vec<R>,
    row_id: Arc<dyn Fn(&R) -> usize>,
    reorderable_rows: bool,
    reorderable_columns: bool,
    default_sort: Option<(&'static str, bool)>,
    on_rows_reorder: Option<RowsHandler>,
    on_columns_reorder: Option<ColumnsHandler>,
    configure: Option<ConfigureHandler>,
}

impl<R: Clone> Clone for DataTable<R> {
    fn clone(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            rows: self.rows.clone(),
            row_id: Arc::clone(&self.row_id),
            reorderable_rows: self.reorderable_rows,
            reorderable_columns: self.reorderable_columns,
            default_sort: self.default_sort,
            on_rows_reorder: self.on_rows_reorder.clone(),
            on_columns_reorder: self.on_columns_reorder.clone(),
            configure: self.configure.clone(),
        }
    }
}

impl<R: PartialEq> PartialEq for DataTable<R> {
    fn eq(&self, other: &Self) -> bool {
        self.columns == other.columns
            && self.rows == other.rows
            && Arc::ptr_eq(&self.row_id, &other.row_id)
            && self.reorderable_rows == other.reorderable_rows
            && self.reorderable_columns == other.reorderable_columns
            && self.default_sort == other.default_sort
            && same_callback(&self.on_rows_reorder, &other.on_rows_reorder)
            && same_callback(&self.on_columns_reorder, &other.on_columns_reorder)
            && same_callback(&self.configure, &other.configure)
    }
}

impl<R: 'static> DataTable<R> {
    /// Create a table. `row_id` returns the stable ID of a row, unique within `rows`.
    ///
    /// Pass `rows` in their natural order. After the first render the table keeps its own
    /// order, which only changes when the user drags a row; rows it has not seen before are
    /// appended at the end.
    pub fn new(
        columns: impl IntoIterator<Item = Column<R>>,
        rows: impl IntoIterator<Item = R>,
        row_id: impl Fn(&R) -> usize + 'static,
    ) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            row_id: Arc::new(row_id),
            reorderable_rows: false,
            reorderable_columns: false,
            default_sort: None,
            on_rows_reorder: None,
            on_columns_reorder: None,
            configure: None,
        }
    }

    /// Let the user drag rows. Disabled while a column sort is active, because a sorted view
    /// has no row order to change.
    #[inline]
    pub fn reorderable_rows(mut self) -> Self {
        self.reorderable_rows = true;
        self
    }

    /// Let the user drag header cells to move columns.
    #[inline]
    pub fn reorderable_columns(mut self) -> Self {
        self.reorderable_columns = true;
        self
    }

    /// The column the natural row order is already sorted by, as `(key, ascending)`.
    ///
    /// Its header is shown as sorted from the start and its first click flips the direction.
    #[inline]
    pub fn default_sort(mut self, key: &'static str, ascending: bool) -> Self {
        self.default_sort = Some((key, ascending));
        self
    }

    /// Called after a row drag with the full row-ID order, including rows that are currently
    /// not passed in (for example filtered out), which keep their place.
    #[inline]
    pub fn on_rows_reorder(mut self, callback: impl Fn(&[usize]) + 'static) -> Self {
        self.on_rows_reorder = Some(Arc::new(callback));
        self
    }

    /// Called after a column drag with the keys in their new display order.
    #[inline]
    pub fn on_columns_reorder(mut self, callback: impl Fn(&[&'static str]) + 'static) -> Self {
        self.on_columns_reorder = Some(Arc::new(callback));
        self
    }

    /// Adjust the underlying [`DndTable`]: density, height, colors, metrics...
    ///
    /// Do not set `columns`, `sort` or the reorder callbacks here; the `DataTable` owns them.
    #[inline]
    pub fn table(mut self, configure: impl Fn(DndTable) -> DndTable + 'static) -> Self {
        self.configure = Some(Arc::new(configure));
        self
    }
}

impl<R: Clone + PartialEq + 'static> Component for DataTable<R> {
    fn render(&self) -> impl IntoElement {
        let keys: Vec<&'static str> = self.columns.iter().map(|column| column.key).collect();
        let present_ids: Vec<usize> = self.rows.iter().map(|row| (self.row_id)(row)).collect();

        let mut column_order = use_state(|| keys.clone());
        let mut row_order = use_state(|| present_ids.clone());
        let sort = use_state(|| None::<(&'static str, bool)>);

        // Follow the declared columns and rows while keeping what the user arranged.
        let order = calc::reconcile_order(&column_order.read(), &keys);
        if *column_order.read() != order {
            column_order.set(order.clone());
        }

        let known_ids = calc::merge_known_order(&row_order.read(), &present_ids);
        if *row_order.read() != known_ids {
            row_order.set(known_ids.clone());
        }

        let ordered: Vec<&Column<R>> = order
            .iter()
            .filter_map(|key| self.columns.iter().find(|column| column.key == *key))
            .collect();

        // --- Rows: natural order, then the active sort -------------------------------------

        let by_id: HashMap<usize, &R> = present_ids.iter().copied().zip(&self.rows).collect();
        let mut view: Vec<(usize, &R)> =
            known_ids.iter().filter_map(|id| by_id.get(id).map(|row| (*id, *row))).collect();

        let active_sort = *sort.read();
        let compare = active_sort.and_then(|(key, ascending)| {
            let compare = ordered.iter().find(|column| column.key == key)?.compare.clone()?;
            Some((compare, ascending))
        });

        if let Some((compare, ascending)) = &compare {
            view.sort_by(|(_, a), (_, b)| {
                let ordering = compare(a, b);
                if *ascending { ordering } else { ordering.reverse() }
            });
        }

        // --- Display <-> key translation: the table speaks display indices ------------------

        let to_display = |(key, ascending): (&str, bool)| -> Option<SortSpec> {
            let display = ordered.iter().position(|column| column.key == key)?;
            Some((display, ascending))
        };

        let table_sort = active_sort.and_then(to_display);
        let default_sort = self.default_sort.and_then(to_display);

        let last = ordered.len().saturating_sub(1);
        let widths: Vec<ColumnWidth> = ordered
            .iter()
            .enumerate()
            .map(|(display, column)| if display == last { ColumnWidth::Fill } else { column.width })
            .collect();

        // --- Callbacks -------------------------------------------------------------------------

        let keys_for_sort = order.clone();
        let sort_state = sort;
        let on_sort = move |next: Option<SortSpec>| {
            let next = next.and_then(|(display, ascending)| {
                keys_for_sort.get(display).map(|key| (*key, ascending))
            });

            let mut state = sort_state;
            state.set(next);
        };

        let mut table = DndTable::new()
            .columns(widths)
            .sort(table_sort)
            .default_sort(default_sort)
            .on_sort(on_sort);

        if self.reorderable_rows && active_sort.is_none() {
            let on_rows_reorder = self.on_rows_reorder.clone();

            table = table.on_reorder(move |from, target, before| {
                let (Some(from), Some(target)) = ({
                    let row_order_state = row_order.read();
                    let position = |id: usize| {
                        row_order_state.as_slice().iter().position(|known| *known == id)
                    };
                    (position(from), position(target))
                }) else {
                    return;
                };

                let mut row_order_state = row_order;
                let mut next = row_order_state.write();
                if calc::move_column(&mut next, from, target, before) {
                    if let Some(callback) = &on_rows_reorder {
                        callback(&next);
                    }
                }
            });
        }

        if self.reorderable_columns {
            let on_columns_reorder = self.on_columns_reorder.clone();

            table = table.on_column_reorder(move |from, target, before| {
                let mut column_order_state = column_order;
                let mut next = column_order_state.write();
                if calc::move_column(&mut next, from, target, before) {
                    if let Some(callback) = &on_columns_reorder {
                        callback(&next);
                    }
                }
            });
        }

        if let Some(configure) = &self.configure {
            table = configure(table);
        }

        // --- Elements -------------------------------------------------------------------------

        let head = DndTableHead::new().child(DndTableRow::new().header(true).children(
            ordered.iter().enumerate().map(|(display, column)| {
                DndTableCell::new()
                    .column(display)
                    .header(true)
                    .title(column.title.clone())
                    .sortable(column.compare.is_some())
                    .sort_direction(column.first_direction)
                    .resizable(column.resizable)
                    .reorderable(self.reorderable_columns)
                    .into_element()
            }),
        ));

        let draggable_rows = self.reorderable_rows && active_sort.is_none();

        let body =
            view.iter().enumerate().fold(DndTableBody::new(), |body, (position, (id, row))| {
                let cells = ordered.iter().enumerate().map(|(display, column)| {
                    DndTableCell::new()
                        .column(display)
                        .resizable(column.resizable)
                        .child((column.render)(row))
                        .into_element()
                });

                let table_row = DndTableRow::new().position(position).children(cells);
                let table_row = if draggable_rows { table_row.id(*id) } else { table_row };

                body.row(*id, table_row.into_element())
            });

        table.child(head).child(body)
    }
}
