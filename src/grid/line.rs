use anyhow::{bail, Result};
use regex::Regex;

use crate::ast::{Call, Expr, Grid};
use crate::util::{arg_usize_expr, expr_to_string};

use super::coords::{mark_origins, resolve_point_call, resolve_point_expr};
use super::LineOptions;

/// Apply directional line placement or ray/pattern marking to the grid.
pub(super) fn grid_mark(mut grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.len() >= 4 && is_line_mode_arg(&call.args[1]) {
        let origins = mark_origins(&grid, &call.args[0])?;
        let dir = expr_to_string(&call.args[2])?;
        let (values, options) = parse_line_tail(&call.args[3..])?;
        return apply_line_values(grid, origins, &dir, values, options);
    }

    match call.args.len() {
        3 => {
            let origins = mark_origins(&grid, &call.args[0])?;
            let ray = expr_to_string(&call.args[1])?;
            let put = expr_to_string(&call.args[2])?;
            let dirs = dirs_from_ray(&ray)?;
            let mut marks = Vec::new();

            for (x, y) in &origins {
                for (dx, dy) in &dirs {
                    let mut cx = *x as isize + dx;
                    let mut cy = *y as isize + dy;
                    while cy >= 0
                        && (cy as usize) < grid.cells.len()
                        && cx >= 0
                        && (cx as usize) < grid.cells[cy as usize].len()
                    {
                        if grid.cells[cy as usize][cx as usize].is_empty() {
                            break;
                        }
                        marks.push((cx as usize, cy as usize));
                        cx += dx;
                        cy += dy;
                    }
                }
            }

            for (x, y) in marks {
                if !origins.contains(&(x, y)) {
                    grid.cells[y][x] = put.clone();
                }
            }

            Ok(grid)
        }
        4 => {
            let origins = mark_origins(&grid, &call.args[0])?;
            let through_pat = Regex::new(&expr_to_string(&call.args[1])?)?;
            let to = expr_to_string(&call.args[2])?;
            let put = expr_to_string(&call.args[3])?;
            let dirs = dirs_from_ray("8")?;
            let mut marks = Vec::new();

            for (x, y) in origins {
                for (dx, dy) in &dirs {
                    let mut cx = x as isize + dx;
                    let mut cy = y as isize + dy;
                    let mut seen = Vec::new();
                    while cy >= 0
                        && (cy as usize) < grid.cells.len()
                        && cx >= 0
                        && (cx as usize) < grid.cells[cy as usize].len()
                    {
                        let cur = &grid.cells[cy as usize][cx as usize];
                        if through_pat.is_match(cur) {
                            seen.push((cx as usize, cy as usize));
                            cx += dx;
                            cy += dy;
                            continue;
                        }
                        if !seen.is_empty() && cur == &to {
                            marks.extend(seen);
                        }
                        break;
                    }
                }
            }

            for (x, y) in marks {
                grid.cells[y][x] = put.clone();
            }

            Ok(grid)
        }
        _ => bail!("mark expects 3 args (from, ray, put) or 4 args (from, through_re, to, put)"),
    }
}

pub(super) fn grid_line(grid: Grid, call: &Call) -> Result<Grid> {
    if call.args.len() < 3 {
        bail!("line expects at least origin, direction, and one value");
    }

    let (origin, dir, values) = match call.args.as_slice() {
        [origin, dir, values @ ..] if matches!(origin, Expr::Call(_)) => {
            (resolve_point_expr(&grid, origin)?, expr_to_string(dir)?, values)
        }
        [_, _, dir, values @ ..] => (resolve_point_call(call, &grid)?, expr_to_string(dir)?, values),
        _ => bail!("line expects either (x, y, dir, values...) or (pick(...), dir, values...)"),
    };

    let (values, options) = parse_line_tail(values)?;
    apply_line_values(grid, vec![origin], &dir, values, options)
}

fn is_line_mode_arg(expr: &Expr) -> bool {
    matches!(expr_to_string(expr).as_deref(), Ok("line" | "ln"))
}

fn apply_line_values(
    mut grid: Grid,
    origins: Vec<(usize, usize)>,
    dir: &str,
    values: Vec<String>,
    options: LineOptions,
) -> Result<Grid> {
    for origin in origins {
        let points = line_points(&grid, origin, dir, values.len(), &options)?;
        for ((x, y), value) in points.into_iter().zip(values.iter()) {
            grid.cells[y][x] = value.clone();
        }
    }
    Ok(grid)
}

/// Parse the value tail of `line(...)`/`mark(..., "line", ...)`.
///
/// Options such as `wrap(...)` and `skip(...)` are accepted only at the end so
/// the actual placed values can stay position-based.
fn parse_line_tail(args: &[Expr]) -> Result<(Vec<String>, LineOptions)> {
    if args.is_empty() {
        bail!("line expects at least one value");
    }

    let mut options = LineOptions::default();
    let mut end = args.len();
    while end > 0 {
        match &args[end - 1] {
            Expr::Call(call) if call.name == "wrap" => {
                if options.wrap_mode.is_some() {
                    bail!("wrap can only be specified once");
                }
                if call.args.len() != 1 {
                    bail!("wrap expects exactly one mode");
                }
                options.wrap_mode = Some(expr_to_string(&call.args[0])?);
                end -= 1;
            }
            Expr::Call(call) if call.name == "skip" => {
                if options.skip != 0 {
                    bail!("skip can only be specified once");
                }
                options.skip = arg_usize_expr(call, 0)?;
                end -= 1;
            }
            _ => break,
        }
    }

    if end == 0 {
        bail!("line expects at least one value before options");
    }

    let values = args[..end]
        .iter()
        .map(expr_to_string)
        .collect::<Result<Vec<_>>>()?;
    Ok((values, options))
}

fn line_points(
    grid: &Grid,
    origin: (usize, usize),
    dir: &str,
    count: usize,
    options: &LineOptions,
) -> Result<Vec<(usize, usize)>> {
    if is_fill_mode(dir) {
        return filled_line_points(grid, origin, dir, count, options.skip);
    }

    if options.skip != 0 {
        bail!("skip(...) is only supported for fill_* line directions");
    }

    if let Some(mode) = options.wrap_mode.as_deref() {
        return wrapped_line_points(grid, origin, dir, count, mode);
    }

    match dir {
        "right" | "r" => ray_points(grid, origin, (1, 0), count),
        "left" | "l" => ray_points(grid, origin, (-1, 0), count),
        "up" | "u" => ray_points(grid, origin, (0, -1), count),
        "down" | "d" => ray_points(grid, origin, (0, 1), count),
        "ur" => ray_points(grid, origin, (1, -1), count),
        "ul" => ray_points(grid, origin, (-1, -1), count),
        "dr" => ray_points(grid, origin, (1, 1), count),
        "dl" => ray_points(grid, origin, (-1, 1), count),
        "horiz" | "h" => centered_points(grid, origin, (1, 0), count),
        "vert" | "v" => centered_points(grid, origin, (0, 1), count),
        "diag_dr" | "xr" => centered_points(grid, origin, (1, 1), count),
        "diag_dl" | "xl" => centered_points(grid, origin, (-1, 1), count),
        _ => bail!("unknown line direction: {dir}"),
    }
}

fn is_fill_mode(dir: &str) -> bool {
    matches!(dir, "fill_ur" | "fur" | "fill_ul" | "ful")
}

fn filled_line_points(
    grid: &Grid,
    origin: (usize, usize),
    dir: &str,
    count: usize,
    skip: usize,
) -> Result<Vec<(usize, usize)>> {
    let positions = match dir {
        "fill_ur" | "fur" => fill_ur_positions(grid, origin),
        "fill_ul" | "ful" => fill_ul_positions(grid, origin),
        _ => bail!("unknown fill direction: {dir}"),
    };

    if skip + count > positions.len() {
        bail!("filled line placement runs out of space");
    }

    Ok(positions[skip..skip + count].to_vec())
}

fn wrapped_line_points(
    grid: &Grid,
    origin: (usize, usize),
    dir: &str,
    count: usize,
    wrap_mode: &str,
) -> Result<Vec<(usize, usize)>> {
    let positions = match (dir, wrap_mode) {
        ("right" | "r", "row") => row_wrap_positions(grid, false),
        ("left" | "l", "row") => row_wrap_positions(grid, true),
        ("down" | "d", "col") => col_wrap_positions(grid, false),
        ("up" | "u", "col") => col_wrap_positions(grid, true),
        ("dr", "diag_dr") => diag_dr_wrap_positions(grid, false),
        ("ul", "diag_dr") => diag_dr_wrap_positions(grid, true),
        ("dl", "diag_dl") => diag_dl_wrap_positions(grid, false),
        ("ur", "diag_dl") => diag_dl_wrap_positions(grid, true),
        ("horiz" | "h" | "vert" | "v" | "diag_dr" | "xr" | "diag_dl" | "xl", _) => {
            bail!("wrap is not supported for centered line directions")
        }
        _ => bail!("wrap({wrap_mode}) is not compatible with line direction {dir}"),
    };

    let start = positions
        .iter()
        .position(|&point| point == origin)
        .ok_or_else(|| anyhow::anyhow!("wrap start point not found"))?;
    if start + count > positions.len() {
        bail!("wrapped line placement runs out of space");
    }

    Ok(positions[start..start + count].to_vec())
}

fn row_wrap_positions(grid: &Grid, reverse: bool) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    for y in 0..grid.cells.len() {
        if reverse {
            for x in (0..grid.cells[y].len()).rev() {
                positions.push((x, y));
            }
        } else {
            for x in 0..grid.cells[y].len() {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn col_wrap_positions(grid: &Grid, reverse: bool) -> Vec<(usize, usize)> {
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut positions = Vec::new();
    for x in 0..width {
        if reverse {
            for y in (0..grid.cells.len()).rev() {
                if x < grid.cells[y].len() {
                    positions.push((x, y));
                }
            }
        } else {
            for y in 0..grid.cells.len() {
                if x < grid.cells[y].len() {
                    positions.push((x, y));
                }
            }
        }
    }
    positions
}

fn diag_dr_wrap_positions(grid: &Grid, reverse: bool) -> Vec<(usize, usize)> {
    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut positions = Vec::new();

    for start_x in 0..width {
        push_diag_dr(grid, (start_x as isize, 0), reverse, &mut positions);
    }
    for start_y in 1..height {
        push_diag_dr(grid, (0, start_y as isize), reverse, &mut positions);
    }

    positions
}

fn diag_dl_wrap_positions(grid: &Grid, reverse: bool) -> Vec<(usize, usize)> {
    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut positions = Vec::new();

    if width == 0 {
        return positions;
    }

    for start_x in (0..width).rev() {
        push_diag_dl(grid, (start_x as isize, 0), reverse, &mut positions);
    }
    let last_x = (width - 1) as isize;
    for start_y in 1..height {
        push_diag_dl(grid, (last_x, start_y as isize), reverse, &mut positions);
    }

    positions
}

fn push_diag_dr(
    grid: &Grid,
    start: (isize, isize),
    reverse: bool,
    out: &mut Vec<(usize, usize)>,
) {
    let mut diag = Vec::new();
    let mut x = start.0;
    let mut y = start.1;
    while x >= 0 && y >= 0 {
        let xi = x as usize;
        let yi = y as usize;
        if yi >= grid.cells.len() {
            break;
        }
        if xi < grid.cells[yi].len() {
            diag.push((xi, yi));
        }
        x += 1;
        y += 1;
    }
    if reverse {
        diag.reverse();
    }
    out.extend(diag);
}

fn push_diag_dl(
    grid: &Grid,
    start: (isize, isize),
    reverse: bool,
    out: &mut Vec<(usize, usize)>,
) {
    let mut diag = Vec::new();
    let mut x = start.0;
    let mut y = start.1;
    while x >= 0 && y >= 0 {
        let xi = x as usize;
        let yi = y as usize;
        if yi >= grid.cells.len() {
            break;
        }
        if xi < grid.cells[yi].len() {
            diag.push((xi, yi));
        }
        x -= 1;
        y += 1;
    }
    if reverse {
        diag.reverse();
    }
    out.extend(diag);
}

fn fill_ur_positions(grid: &Grid, origin: (usize, usize)) -> Vec<(usize, usize)> {
    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut positions = Vec::new();

    for diag in 0..(width + height) {
        let mut added = false;
        for dx in 0..=diag {
            let dy = diag - dx;
            let x = origin.0 + dx;
            let y = origin.1 + dy;
            if y >= grid.cells.len() {
                continue;
            }
            if x < grid.cells[y].len() {
                positions.push((x, y));
                added = true;
            }
        }
        if !added && diag > width + height {
            break;
        }
    }

    positions
}

fn fill_ul_positions(grid: &Grid, origin: (usize, usize)) -> Vec<(usize, usize)> {
    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut positions = Vec::new();

    for diag in 0..(width + height) {
        let mut added = false;
        for dx in 0..=diag {
            let dy = diag - dx;
            if origin.0 < dx {
                continue;
            }
            let x = origin.0 - dx;
            let y = origin.1 + dy;
            if y >= grid.cells.len() {
                continue;
            }
            if x < grid.cells[y].len() {
                positions.push((x, y));
                added = true;
            }
        }
        if !added && diag > width + height {
            break;
        }
    }

    positions
}

fn ray_points(
    grid: &Grid,
    origin: (usize, usize),
    delta: (isize, isize),
    count: usize,
) -> Result<Vec<(usize, usize)>> {
    let mut points = Vec::with_capacity(count);
    for step in 0..count {
        points.push(offset_point(grid, origin, delta, step as isize)?);
    }
    Ok(points)
}

fn centered_points(
    grid: &Grid,
    origin: (usize, usize),
    step_delta: (isize, isize),
    count: usize,
) -> Result<Vec<(usize, usize)>> {
    if count % 2 == 0 {
        bail!("centered line directions require an odd number of values");
    }

    let mid = (count / 2) as isize;
    let mut points = Vec::with_capacity(count);
    for idx in 0..count {
        let offset = idx as isize - mid;
        points.push(offset_point(grid, origin, step_delta, offset)?);
    }
    Ok(points)
}

fn offset_point(
    grid: &Grid,
    origin: (usize, usize),
    delta: (isize, isize),
    step: isize,
) -> Result<(usize, usize)> {
    let x = origin.0 as isize + delta.0 * step;
    let y = origin.1 as isize + delta.1 * step;
    if x < 0 || y < 0 {
        bail!("line placement goes out of bounds");
    }

    let yi = y as usize;
    if yi >= grid.cells.len() {
        bail!("line placement goes out of bounds");
    }

    let xi = x as usize;
    if xi >= grid.cells[yi].len() {
        bail!("line placement goes out of bounds");
    }

    Ok((xi, yi))
}

fn dirs_from_ray(ray: &str) -> Result<Vec<(isize, isize)>> {
    match ray {
        "orth" => Ok(vec![(1, 0), (-1, 0), (0, 1), (0, -1)]),
        "diag" => Ok(vec![(1, 1), (1, -1), (-1, 1), (-1, -1)]),
        "alldir" | "8" => Ok(vec![
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ]),
        _ => bail!("unknown ray: {ray}"),
    }
}
