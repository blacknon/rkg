use anyhow::{bail, Result};

use crate::ast::{Call, Expr, Grid};
use crate::util::{arg_usize_expr, expr_to_string};

pub(super) fn mark_origins(grid: &Grid, expr: &Expr) -> Result<Vec<(usize, usize)>> {
    match expr {
        Expr::Call(call) if is_origin_call(call) => Ok(vec![resolve_origin_call(grid, call)?]),
        _ => {
            let from = expr_to_string(expr)?;
            let mut origins = Vec::new();
            for y in 0..grid.cells.len() {
                for x in 0..grid.cells[y].len() {
                    if grid.cells[y][x] == from {
                        origins.push((x, y));
                    }
                }
            }
            if origins.is_empty() {
                bail!("mark origin not found: {from}");
            }
            Ok(origins)
        }
    }
}

pub(super) fn resolve_point_expr(grid: &Grid, expr: &Expr) -> Result<(usize, usize)> {
    match expr {
        Expr::Call(call) if is_origin_call(call) => resolve_origin_call(grid, call),
        _ => bail!("expected pick(...), p(...), point(...), or pt(...) for point argument"),
    }
}

pub(super) fn resolve_point_call(call: &Call, grid: &Grid) -> Result<(usize, usize)> {
    let x = arg_usize_expr(call, 0)?;
    let y = arg_usize_expr(call, 1)?;
    point_to_index(grid, x, y)
}

fn is_origin_call(call: &Call) -> bool {
    is_pick_call(call) || is_point_call(call)
}

fn is_pick_call(call: &Call) -> bool {
    call.name == "pick" || call.name == "p"
}

fn is_point_call(call: &Call) -> bool {
    call.name == "point" || call.name == "pt"
}

fn pick_coord(grid: &Grid, call: &Call) -> Result<(usize, usize)> {
    if call.args.is_empty() || call.args.len() > 2 {
        bail!("pick/p expects 1 or 2 args (value, nth)");
    }

    let needle = expr_to_string(&call.args[0])?;
    let nth = if call.args.len() == 2 {
        arg_usize_expr(call, 1)?
    } else {
        1
    };

    let mut seen = 0usize;
    for y in 0..grid.cells.len() {
        for x in 0..grid.cells[y].len() {
            if grid.cells[y][x] == needle {
                seen += 1;
                if seen == nth {
                    return Ok((x, y));
                }
            }
        }
    }

    bail!("pick could not find match #{nth} for {needle}")
}

fn resolve_origin_call(grid: &Grid, call: &Call) -> Result<(usize, usize)> {
    if is_pick_call(call) {
        return pick_coord(grid, call);
    }
    if is_point_call(call) {
        return point_coord(grid, call);
    }
    bail!("expected pick(...), p(...), point(...), or pt(...)")
}

fn point_coord(grid: &Grid, call: &Call) -> Result<(usize, usize)> {
    if call.args.len() != 2 {
        bail!("point/pt expects exactly 2 args (x, y)");
    }
    let x = arg_usize_expr(call, 0)?;
    let y = arg_usize_expr(call, 1)?;
    point_to_index(grid, x, y)
}

fn point_to_index(grid: &Grid, x: usize, y: usize) -> Result<(usize, usize)> {
    if x == 0 || y == 0 {
        bail!("grid coordinates are 1-based");
    }

    let yi = y - 1;
    if yi >= grid.cells.len() {
        bail!("grid y out of range: {y}");
    }

    let xi = x - 1;
    if xi >= grid.cells[yi].len() {
        bail!("grid x out of range at row {y}: {x}");
    }

    Ok((xi, yi))
}
