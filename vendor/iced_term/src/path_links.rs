use alacritty_terminal::grid::{Dimensions, Grid};
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use std::ops::{Range, RangeInclusive};

pub(super) fn path_at(
    grid: &Grid<Cell>,
    point: Point,
) -> Option<(RangeInclusive<Point>, String)> {
    if point.line < grid.topmost_line()
        || point.line > grid.bottommost_line()
        || point.column >= grid.columns()
    {
        return None;
    }
    let wraps = |row: Line| {
        grid[row][grid.last_column()]
            .flags
            .contains(Flags::WRAPLINE)
    };
    let mut first_row = point.line;
    let mut last_row = point.line;
    while first_row > grid.topmost_line() && wraps(first_row - 1i32) {
        first_row -= 1i32;
        if point.line.0 - first_row.0 > 100 {
            return None;
        }
    }
    while last_row < grid.bottommost_line() && wraps(last_row) {
        last_row += 1i32;
        if last_row.0 - first_row.0 > 100 {
            return None;
        }
    }
    let mut line = String::new();
    let mut cells = Vec::new();

    // Keep the byte-to-cell mapping across soft wraps and wide characters.
    for row in first_row.0..=last_row.0 {
        for column in 0..grid.columns() {
            let position = Point::new(Line(row), Column(column));
            let cell = &grid[position];
            if !cell.flags.intersects(
                Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            ) {
                cells.push((line.len(), position));
                line.push(cell.c);
                line.extend(cell.zerowidth().into_iter().flatten().copied());
            }
        }
    }

    let hit = cells
        .partition_point(|&(_, position)| position <= point)
        .checked_sub(1)?;
    let range = path_in_line(&line, cells[hit].0)?;
    let first = cells.partition_point(|&(offset, _)| offset < range.start);
    let last = cells.partition_point(|&(offset, _)| offset < range.end) - 1;
    Some((cells[first].1..=cells[last].1, line[range].to_owned()))
}

fn path_in_line(line: &str, hit: usize) -> Option<Range<usize>> {
    for quote in ['"', '\'', '`'] {
        let mut quotes = line.match_indices(quote);
        while let (Some((start, _)), Some((end, _))) =
            (quotes.next(), quotes.next())
        {
            let range = start + 1..end;
            if range.contains(&hit) && looks_like_path(&line[range.clone()]) {
                return Some(range);
            }
        }
    }

    // Tool diagnostics commonly print `--label:/path with spaces` or `/path: status`.
    let diagnostic_path = line.split_once(':').and_then(|(prefix, rest)| {
        let label = prefix.trim_start().strip_prefix("--")?;
        (!label.is_empty()
            && label
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
            && rest.starts_with('/'))
        .then_some(rest)
    });
    let body = diagnostic_path.unwrap_or(line);
    if let Some(path) = body
        .split_once(": ")
        .map(|(path, _)| path)
        .or(diagnostic_path)
    {
        let path = path.trim();
        let start = line.len() - body.trim_start().len();
        let range = start..start + path.len();
        if range.contains(&hit)
            && looks_like_path(path)
            && path.split_once('/').is_some_and(|(prefix, _)| {
                !prefix.contains(char::is_whitespace)
            })
        {
            return Some(range);
        }
    }

    let separator = |ch: char| {
        ch.is_whitespace() || ":<>=\"'`|()[]{},;，。；：！？、".contains(ch)
    };
    let start = line[..hit].rfind(separator).map_or(0, |index| {
        index + line[index..].chars().next().unwrap().len_utf8()
    });
    let end = line[hit..]
        .find(separator)
        .map_or(line.len(), |index| hit + index);
    let path = line[start..end].trim_end_matches(['.', '!', '?']);
    let range = start..start + path.len();
    (range.contains(&hit) && looks_like_path(path)).then_some(range)
}

fn looks_like_path(path: &str) -> bool {
    path.len() > 1
        && path.contains('/')
        && !path.starts_with("//")
        && !path.contains("://")
        && !path.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "path_links_tests.rs"]
mod tests;
