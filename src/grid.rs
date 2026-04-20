use anyhow::{bail, Result};
use regex::Regex;

use crate::ast::{Call, Grid, GridConfig};
use crate::util::{arg_string, expr_to_string, split_keep_nonempty, unescape};

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
        "transpose" | "t" => grid_transpose(grid),
        "rotate" | "rt" => grid_rotate(grid, call),
        "mark" | "m" => grid_mark(grid, call),
        other => bail!("unknown grid method: {other}"),
    }
}

fn grid_transpose(mut grid: Grid) -> Result<Grid> {
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

fn grid_rotate(mut grid: Grid, call: &Call) -> Result<Grid> {
    let dir = arg_string(call, 0)?;
    let height = grid.cells.len();
    let width = grid.cells.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut padded = vec![vec![String::new(); width]; height];

    for y in 0..height {
        for x in 0..grid.cells[y].len() {
            padded[y][x] = grid.cells[y][x].clone();
        }
    }

    grid.cells = match dir.as_str() {
        "r" | "right" => {
            let mut out = vec![vec![String::new(); height]; width];
            for y in 0..height {
                for x in 0..width {
                    out[x][height - 1 - y] = padded[y][x].clone();
                }
            }
            out
        }
        "l" | "left" => {
            let mut out = vec![vec![String::new(); height]; width];
            for y in 0..height {
                for x in 0..width {
                    out[width - 1 - x][y] = padded[y][x].clone();
                }
            }
            out
        }
        "180" => {
            let mut out = vec![vec![String::new(); width]; height];
            for y in 0..height {
                for x in 0..width {
                    out[height - 1 - y][width - 1 - x] = padded[y][x].clone();
                }
            }
            out
        }
        _ => bail!("rotate direction must be r, l, or 180"),
    };

    Ok(grid)
}

fn grid_mark(mut grid: Grid, call: &Call) -> Result<Grid> {
    match call.args.len() {
        3 => {
            let from = expr_to_string(&call.args[0])?;
            let ray = expr_to_string(&call.args[1])?;
            let put = expr_to_string(&call.args[2])?;
            let dirs = dirs_from_ray(&ray)?;
            let mut marks = Vec::new();

            for y in 0..grid.cells.len() {
                for x in 0..grid.cells[y].len() {
                    if grid.cells[y][x] == from {
                        for (dx, dy) in &dirs {
                            let mut cx = x as isize + dx;
                            let mut cy = y as isize + dy;
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
                }
            }

            for (x, y) in marks {
                if grid.cells[y][x] != from {
                    grid.cells[y][x] = put.clone();
                }
            }

            Ok(grid)
        }
        4 => {
            let from = expr_to_string(&call.args[0])?;
            let through_pat = Regex::new(&expr_to_string(&call.args[1])?)?;
            let to = expr_to_string(&call.args[2])?;
            let put = expr_to_string(&call.args[3])?;
            let dirs = dirs_from_ray("8")?;
            let mut marks = Vec::new();

            for y in 0..grid.cells.len() {
                for x in 0..grid.cells[y].len() {
                    if grid.cells[y][x] == from {
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

fn dirs_from_ray(ray: &str) -> Result<Vec<(isize, isize)>> {
    match ray {
        "rook" | "rk" => Ok(vec![(1, 0), (-1, 0), (0, 1), (0, -1)]),
        "bishop" | "bp" => Ok(vec![(1, 1), (1, -1), (-1, 1), (-1, -1)]),
        "queen" | "q" | "8" => Ok(vec![
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
