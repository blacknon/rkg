use anyhow::{bail, Result};

use crate::ast::{Call, Grid, GridConfig};
use crate::util::{arg_string, split_keep_nonempty, unescape};

mod coords;
mod line;
mod ops;

#[derive(Default)]
pub(crate) struct LineOptions {
    pub(crate) wrap_mode: Option<String>,
    pub(crate) skip: usize,
}

/// Parse the current input into a rectangular-ish cell grid.
///
/// When `fs` is set, each row is split by that separator. Otherwise the input
/// is treated as a character grid.
pub fn parse_grid(input: &str, cfg: GridConfig) -> Result<Grid> {
    let rs = unescape(&cfg.rs);
    let mut cells = Vec::new();

    for raw in split_keep_nonempty(input, &rs) {
        let line = raw.trim_end_matches('\r');
        if let Some(fs) = &cfg.fs {
            cells.push(line.split(&unescape(fs)).map(|s| s.to_string()).collect());
        } else {
            cells.push(line.chars().map(|c| c.to_string()).collect());
        }
    }

    Ok(Grid { cells, cfg })
}

/// Render the grid using the currently active output separators.
pub fn render_grid(grid: &Grid) -> String {
    let mut out = String::new();
    let ors = unescape(&grid.cfg.ors);
    let ofs = unescape(&grid.cfg.ofs);
    for row in &grid.cells {
        out.push_str(&row.join(&ofs));
        out.push_str(&ors);
    }
    out
}

/// Apply one DSL call to the current grid value.
pub fn apply_grid_call(mut grid: Grid, call: &Call) -> Result<Grid> {
    match call.name.as_str() {
        "fs" => {
            grid.cfg.fs = Some(arg_string(call, 0)?);
            parse_grid(&render_grid(&grid), grid.cfg.clone())
        }
        "rs" => {
            grid.cfg.rs = arg_string(call, 0)?;
            parse_grid(&render_grid(&grid), grid.cfg.clone())
        }
        "ofs" => {
            grid.cfg.ofs = arg_string(call, 0)?;
            Ok(grid)
        }
        "ors" => {
            grid.cfg.ors = arg_string(call, 0)?;
            Ok(grid)
        }
        "transpose" | "t" => ops::grid_transpose(grid, call),
        "rotate" | "rt" => ops::grid_rotate(grid, call),
        "pad" | "pd" => ops::grid_pad(grid, call),
        "align" | "al" => ops::grid_align(grid, call),
        "reverse" | "rev" | "rv" => ops::grid_reverse(grid, call),
        "get" => ops::grid_get(grid, call),
        "set" => ops::grid_set(grid, call),
        "line" | "ln" => line::grid_line(grid, call),
        "mark" | "m" => line::grid_mark(grid, call),
        other => bail!("unknown grid method: {other}"),
    }
}
