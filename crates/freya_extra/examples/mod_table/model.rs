use std::cmp::Ordering;

use freya_extra::dnd_table::SortSpec;

#[derive(Clone, PartialEq)]
pub struct ModRow {
    pub id: usize,
    pub priority: usize,
    pub enabled: bool,
    pub name: String,
    pub mod_type: String,
    pub site: String,
}

pub const COLUMN_TITLES: [&str; 6] = ["Enabled", "ID", "Mod Name", "Type", "URL", "Priority"];

pub fn initial_rows() -> Vec<ModRow> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterColumn {
    Name,
    Type,
    Site,
    Enabled,
}

impl FilterColumn {
    pub fn title(self) -> &'static str {
        match self {
            Self::Name => "Mod Name",
            Self::Type => "Type",
            Self::Site => "URL",
            Self::Enabled => "Enabled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterOperator {
    Contains,
    Equals,
    StartsWith,
}

impl FilterOperator {
    pub fn title(self) -> &'static str {
        match self {
            Self::Contains => "Contains",
            Self::Equals => "Equals",
            Self::StartsWith => "Starts with",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnFilter {
    pub column: FilterColumn,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Clone)]
pub struct Query {
    pub text: String,
    pub filters: Vec<ColumnFilter>,
    pub enabled_only: bool,
    pub sort: Option<SortSpec>,
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

fn matches_filter(row: &ModRow, filter: &ColumnFilter) -> bool {
    let value = filter.value.trim();

    if value.is_empty() {
        return true;
    }

    let actual = match filter.column {
        FilterColumn::Name => row.name.as_str(),
        FilterColumn::Type => row.mod_type.as_str(),
        FilterColumn::Site => row.site.as_str(),
        FilterColumn::Enabled => {
            return match filter.operator {
                FilterOperator::Contains => {
                    "enabled".contains(&value.to_lowercase()) && row.enabled
                        || "disabled".contains(&value.to_lowercase()) && !row.enabled
                }
                FilterOperator::Equals => {
                    value.eq_ignore_ascii_case(if row.enabled { "enabled" } else { "disabled" })
                        || value == if row.enabled { "true" } else { "false" }
                }
                FilterOperator::StartsWith => (if row.enabled { "enabled" } else { "disabled" })
                    .starts_with(&value.to_lowercase()),
            };
        }
    };

    let actual = actual.to_lowercase();
    let expected = value.to_lowercase();

    match filter.operator {
        FilterOperator::Contains => actual.contains(&expected),
        FilterOperator::Equals => actual == expected,
        FilterOperator::StartsWith => actual.starts_with(&expected),
    }
}

pub fn matches_query(row: &ModRow, query: &Query) -> bool {
    if query.enabled_only && !row.enabled {
        return false;
    }

    let search_matches = query.text.is_empty()
        || [row.name.as_str(), row.mod_type.as_str(), row.site.as_str()]
            .iter()
            .any(|field| field.to_lowercase().contains(&query.text));

    search_matches && query.filters.iter().all(|filter| matches_filter(row, filter))
}

pub fn build_view<'a>(rows: &'a [ModRow], query: &Query) -> Vec<&'a ModRow> {
    let mut view: Vec<&ModRow> = rows.iter().filter(|row| matches_query(row, query)).collect();

    if let Some((column, ascending)) = query.sort {
        view.sort_by(|a, b| {
            let ordering = compare_by_column(column, a, b);
            if ascending { ordering } else { ordering.reverse() }
        });
    }

    view
}

pub fn normalize_priorities(rows: &mut [ModRow]) {
    for (index, row) in rows.iter_mut().enumerate() {
        row.priority = index + 1;
    }
}

pub fn reorder_visible(
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

    let target_key = rows[target_index].priority as i64 * 2;
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

pub fn apply_enabled(rows: &mut [ModRow], scope: Option<&[usize]>, enabled: bool) {
    for row in rows {
        if scope.is_none_or(|ids| ids.contains(&row.id)) {
            row.enabled = enabled;
        }
    }
}
