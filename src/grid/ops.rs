use anyhow::{bail, Context, Result};

use crate::ast::{Call, Expr, Grid};
use crate::util::{arg_string, expr_to_string};

use super::coords::{resolve_point_call, resolve_point_expr};

pub(super) fn grid_transpose(mut grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.len() > 1 {
        bail!("transpose expects 0 or 1 args: optional pad(...)");
    }

    if let Some(arg) = call.args.first() {
        let pad_value = parse_optional_pad_call(arg, "transpose")?;
        pad_grid_rows(&mut grid, &pad_value);
    }

    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut out = vec![vec![String::new(); height]; width];

    for y in 0..height {
        for x in 0..grid.cells[y].len() {
            out[x][y] = grid.cells[y][x].clone();
        }
    }

    grid.cells = out;
    Ok(grid)
}

pub(super) fn grid_rotate(mut grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.is_empty() || call.args.len() > 2 {
        bail!("rotate expects 1 or 2 args: direction, optional pad(...)");
    }

    let dir = arg_string(call, 0)?;
    if let Some(arg) = call.args.get(1) {
        let pad_value = parse_optional_pad_call(arg, "rotate")?;
        pad_grid_rows(&mut grid, &pad_value);
    }

    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut padded = vec![vec![String::new(); width]; height];

    for y in 0..height {
        for x in 0..grid.cells[y].len() {
            padded[y][x] = grid.cells[y][x].clone();
        }
    }

    grid.cells = match dir.as_str() {
        "r" | "right" => rotate_right(&padded, height, width),
        "l" | "left" => rotate_left(&padded, height, width),
        "180" => rotate_180(&padded, height, width),
        _ => bail!("rotate direction must be r, l, or 180"),
    };

    Ok(grid)
}

pub(super) fn grid_reverse(mut grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.is_empty() || call.args.len() > 2 {
        bail!("rev expects 1 or 2 args: mode, optional pad(...)");
    }

    let mode = expr_to_string(&call.args[0])?;
    if let Some(arg) = call.args.get(1) {
        let pad_value = parse_optional_pad_call(arg, "rev")?;
        pad_grid_rows(&mut grid, &pad_value);
    }

    match mode.as_str() {
        "h" | "horizontal" => {
            for row in &mut grid.cells {
                row.reverse();
            }
        }
        "v" | "vertical" => grid.cells.reverse(),
        "hv" | "vh" | "180" => {
            for row in &mut grid.cells {
                row.reverse();
            }
            grid.cells.reverse();
        }
        _ => bail!("rev mode must be h, v, hv, or 180"),
    }

    Ok(grid)
}

pub(super) fn grid_align(mut grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.is_empty() || call.args.len() > 2 {
        bail!("align expects 1 or 2 args: mode, optional pad(...)");
    }

    let mode = expr_to_string(&call.args[0])?;
    let pad = match call.args.get(1) {
        Some(arg) => parse_optional_pad_call(arg, "align")?,
        None => " ".to_string(),
    };

    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    for row in &mut grid.cells {
        let missing = width.saturating_sub(row.len());
        if missing == 0 {
            continue;
        }

        let (left, right) = match mode.as_str() {
            "left" | "l" => (0, missing),
            "center" | "c" => (missing / 2, missing - (missing / 2)),
            "right" | "r" => (missing, 0),
            _ => bail!("align mode must be left, center, right, l, c, or r"),
        };

        let mut out = Vec::with_capacity(width);
        out.extend(std::iter::repeat_n(pad.clone(), left));
        out.extend(row.iter().cloned());
        out.extend(std::iter::repeat_n(pad.clone(), right));
        *row = out;
    }

    Ok(grid)
}

pub(super) fn grid_pad(mut grid: Grid, call: &Call) -> Result<Grid> {
    let (top, bottom, left, right, pad_value) = parse_grid_pad_args(call)?;
    if grid.cells.is_empty() {
        return Ok(grid);
    }

    pad_grid_rows(&mut grid, &pad_value);
    let inner_width = grid.cells.first().map(|row| row.len()).unwrap_or(0);
    let total_width = left + inner_width + right;

    for row in &mut grid.cells {
        let mut out = Vec::with_capacity(total_width);
        out.extend(std::iter::repeat_n(pad_value.clone(), left));
        out.extend(row.iter().cloned());
        out.extend(std::iter::repeat_n(pad_value.clone(), right));
        *row = out;
    }

    let border_row = vec![pad_value.clone(); total_width];
    let mut out = Vec::with_capacity(top + grid.cells.len() + bottom);
    out.extend(std::iter::repeat_n(border_row.clone(), top));
    out.extend(grid.cells);
    out.extend(std::iter::repeat_n(border_row, bottom));
    grid.cells = out;
    Ok(grid)
}

pub(super) fn grid_get(mut grid: Grid, call: &Call) -> Result<Grid> {
    let (x, y) = match call.args.as_slice() {
        [arg] => resolve_point_expr(&grid, arg)?,
        [_, _] => resolve_point_call(call, &grid)?,
        _ => bail!("get expects either (x, y) or (pick(...))"),
    };
    let cell = grid.cells[y][x].clone();
    grid.cells = vec![vec![cell]];
    Ok(grid)
}

pub(super) fn grid_set(mut grid: Grid, call: &Call) -> Result<Grid> {
    let ((x, y), value) = match call.args.as_slice() {
        [origin, value] => (resolve_point_expr(&grid, origin)?, expr_to_string(value)?),
        [_, _, value] => (resolve_point_call(call, &grid)?, expr_to_string(value)?),
        _ => bail!("set expects either (x, y, value) or (pick(...), value)"),
    };
    grid.cells[y][x] = value;
    Ok(grid)
}

pub(super) fn pad_grid_rows(grid: &mut Grid, pad_value: &str) {
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    for row in &mut grid.cells {
        while row.len() < width {
            row.push(pad_value.to_string());
        }
    }
}

fn parse_grid_pad_args(call: &Call) -> Result<(usize, usize, usize, usize, String)> {
    let default_pad = " ".to_string();
    match call.args.as_slice() {
        [n] => {
            let n = parse_nonnegative_usize_expr(n, &call.name)?;
            Ok((n, n, n, n, default_pad))
        }
        [a, b, c, d] => Ok((
            parse_nonnegative_usize_expr(a, &call.name)?,
            parse_nonnegative_usize_expr(b, &call.name)?,
            parse_nonnegative_usize_expr(c, &call.name)?,
            parse_nonnegative_usize_expr(d, &call.name)?,
            default_pad,
        )),
        [n, value] if !matches!(value, Expr::Num(_)) => {
            let n = parse_nonnegative_usize_expr(n, &call.name)?;
            Ok((n, n, n, n, expr_to_string(value)?))
        }
        [a, b, c, d, value] => Ok((
            parse_nonnegative_usize_expr(a, &call.name)?,
            parse_nonnegative_usize_expr(b, &call.name)?,
            parse_nonnegative_usize_expr(c, &call.name)?,
            parse_nonnegative_usize_expr(d, &call.name)?,
            expr_to_string(value)?,
        )),
        _ => bail!("pad expects (n, value?) or (top, bottom, left, right, value?)"),
    }
}

fn parse_nonnegative_usize_expr(expr: &Expr, name: &str) -> Result<usize> {
    match expr {
        Expr::Num(n) if *n >= 0 => Ok(*n as usize),
        Expr::Str(s) | Expr::Ident(s) => s
            .parse::<usize>()
            .with_context(|| format!("expected non-negative integer arg for {name}")),
        _ => bail!("expected non-negative integer arg for {name}"),
    }
}

fn parse_optional_pad_call(arg: &Expr, opname: &str) -> Result<String> {
    match arg {
        Expr::Call(pad_call) if pad_call.name == "pad" => {
            if pad_call.args.len() != 1 {
                bail!("pad expects exactly one value");
            }
            expr_to_string(&pad_call.args[0])
        }
        _ => bail!("{opname} optional arg must be pad(...)"),
    }
}

fn rotate_right(padded: &[Vec<String>], height: usize, width: usize) -> Vec<Vec<String>> {
    let mut out = vec![vec![String::new(); height]; width];
    for y in 0..height {
        for x in 0..width {
            out[x][height - 1 - y] = padded[y][x].clone();
        }
    }
    out
}

fn rotate_left(padded: &[Vec<String>], height: usize, width: usize) -> Vec<Vec<String>> {
    let mut out = vec![vec![String::new(); height]; width];
    for y in 0..height {
        for x in 0..width {
            out[width - 1 - x][y] = padded[y][x].clone();
        }
    }
    out
}

fn rotate_180(padded: &[Vec<String>], height: usize, width: usize) -> Vec<Vec<String>> {
    let mut out = vec![vec![String::new(); width]; height];
    for y in 0..height {
        for x in 0..width {
            out[height - 1 - y][width - 1 - x] = padded[y][x].clone();
        }
    }
    out
}
