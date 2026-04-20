use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(name = "regrid")]
#[command(about = "Record/grid DSL processor")]
struct Cli {
    /// Print every statement result separated by ---
    #[arg(long)]
    print_all: bool,

    /// DSL program, e.g. r.fs(",").x(2,";").g(1,s(2)); d.t().rt("r")
    expr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Receiver {
    Rec,
    Grid,
}

#[derive(Debug, Clone)]
struct Statement {
    receiver: Receiver,
    calls: Vec<Call>,
}

#[derive(Debug, Clone)]
struct Call {
    name: String,
    args: Vec<Expr>,
}

#[derive(Debug, Clone)]
enum Expr {
    Str(String),
    Num(i64),
    Ident(String),
    Call(Call),
}

#[derive(Debug, Clone)]
struct RecConfig {
    fs: String,
    rs: String,
    ofs: String,
    ors: String,
}

impl Default for RecConfig {
    fn default() -> Self {
        Self {
            fs: r"\s+".to_string(),
            rs: "\n".to_string(),
            ofs: " ".to_string(),
            ors: "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct GridConfig {
    fs: Option<String>,
    rs: String,
    ofs: String,
    ors: String,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            fs: None,
            rs: "\n".to_string(),
            ofs: "".to_string(),
            ors: "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Records {
    rows: Vec<Vec<String>>,
    cfg: RecConfig,
}

#[derive(Debug, Clone)]
struct Grid {
    cells: Vec<Vec<String>>,
    cfg: GridConfig,
}

#[derive(Debug, Clone)]
enum Agg {
    Sum(usize),
    Count,
    Min(usize),
    Max(usize),
    Avg(usize),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let stmts = parse_program(&cli.expr)?;
    if stmts.is_empty() {
        bail!("empty program");
    }

    let mut outputs = Vec::new();
    for stmt in &stmts {
        let out = eval_statement(stmt, &input)?;
        outputs.push(out);
    }

    if cli.print_all {
        for (i, out) in outputs.iter().enumerate() {
            if i > 0 {
                println!("---");
            }
            print!("{}", out);
        }
    } else if let Some(last) = outputs.last() {
        print!("{}", last);
    }
    Ok(())
}

fn eval_statement(stmt: &Statement, input: &str) -> Result<String> {
    match stmt.receiver {
        Receiver::Rec => {
            let mut rec = parse_records(input, RecConfig::default())?;
            for call in &stmt.calls {
                rec = apply_rec_call(rec, call)?;
            }
            Ok(render_records(&rec))
        }
        Receiver::Grid => {
            let mut grid = parse_grid(input, GridConfig::default())?;
            for call in &stmt.calls {
                grid = apply_grid_call(grid, call)?;
            }
            Ok(render_grid(&grid))
        }
    }
}

fn parse_records(input: &str, cfg: RecConfig) -> Result<Records> {
    let rs = unescape(&cfg.rs);
    let fs_re = Regex::new(&cfg.fs).with_context(|| format!("invalid FS regex: {}", cfg.fs))?;
    let mut rows = Vec::new();
    for raw in split_keep_nonempty(input, &rs) {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let cols: Vec<String> = if cfg.fs == r"\s+" {
            line.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            fs_re
                .split(line)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        };
        rows.push(cols);
    }
    Ok(Records { rows, cfg })
}

fn render_records(rec: &Records) -> String {
    let mut out = String::new();
    let ors = unescape(&rec.cfg.ors);
    for row in &rec.rows {
        out.push_str(&row.join(&unescape(&rec.cfg.ofs)));
        out.push_str(&ors);
    }
    out
}

fn parse_grid(input: &str, cfg: GridConfig) -> Result<Grid> {
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

fn render_grid(grid: &Grid) -> String {
    let mut out = String::new();
    let ors = unescape(&grid.cfg.ors);
    let ofs = unescape(&grid.cfg.ofs);
    for row in &grid.cells {
        out.push_str(&row.join(&ofs));
        out.push_str(&ors);
    }
    out
}

fn apply_rec_call(mut rec: Records, call: &Call) -> Result<Records> {
    match call.name.as_str() {
        "fs" => {
            rec.cfg.fs = arg_string(call, 0)?;
            rec = parse_records(&render_records(&rec), rec.cfg.clone())?;
            Ok(rec)
        }
        "rs" => {
            rec.cfg.rs = arg_string(call, 0)?;
            rec = parse_records(&render_records(&rec), rec.cfg.clone())?;
            Ok(rec)
        }
        "ofs" => {
            rec.cfg.ofs = arg_string(call, 0)?;
            Ok(rec)
        }
        "ors" => {
            rec.cfg.ors = arg_string(call, 0)?;
            Ok(rec)
        }
        "select" | "p" => rec_select(rec, &call.args),
        "replace" | "sb" => rec_replace(rec, call),
        "enum" | "n" => rec_enum(rec, call),
        "explode" | "x" => rec_explode(rec, call),
        "implode" | "i" => rec_implode(rec, call),
        "groupby" | "g" => rec_groupby(rec, call),
        "reshape" | "sh" => rec_reshape(rec, call),
        "flatten" | "f" => rec_flatten_as_records(rec, call),
        other => bail!("unknown rec method: {other}"),
    }
}

fn apply_grid_call(mut grid: Grid, call: &Call) -> Result<Grid> {
    match call.name.as_str() {
        "fs" => {
            grid.cfg.fs = Some(arg_string(call, 0)?);
            grid = parse_grid(&render_grid(&grid), grid.cfg.clone())?;
            Ok(grid)
        }
        "rs" => {
            grid.cfg.rs = arg_string(call, 0)?;
            grid = parse_grid(&render_grid(&grid), grid.cfg.clone())?;
            Ok(grid)
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

fn rec_select(mut rec: Records, args: &[Expr]) -> Result<Records> {
    let specs: Result<Vec<String>> = args.iter().map(expr_to_string).collect();
    let specs = specs?;
    let mut new_rows = Vec::with_capacity(rec.rows.len());
    for row in &rec.rows {
        let mut out = Vec::new();
        for spec in &specs {
            let idxs = expand_spec(spec, row.len())?;
            for idx in idxs {
                if idx < row.len() {
                    out.push(row[idx].clone());
                }
            }
        }
        new_rows.push(out);
    }
    rec.rows = new_rows;
    Ok(rec)
}

fn rec_replace(mut rec: Records, call: &Call) -> Result<Records> {
    let pat = arg_string(call, 0)?;
    let rep = arg_string(call, 1)?;
    let re = Regex::new(&pat).with_context(|| format!("invalid regex: {pat}"))?;
    for row in &mut rec.rows {
        for cell in row {
            *cell = re.replace_all(cell, rep.as_str()).to_string();
        }
    }
    Ok(rec)
}

fn rec_enum(mut rec: Records, call: &Call) -> Result<Records> {
    let arg = arg_string(call, 0)?;
    for (i, row) in rec.rows.iter_mut().enumerate() {
        let label = if arg.eq_ignore_ascii_case("A-Z") {
            let c = ((i % 26) as u8 + b'A') as char;
            c.to_string()
        } else {
            let start: i64 = arg
                .parse()
                .with_context(|| format!("enum start must be integer or A-Z: {arg}"))?;
            (start + i as i64).to_string()
        };
        row.insert(0, label);
    }
    Ok(rec)
}

fn rec_explode(mut rec: Records, call: &Call) -> Result<Records> {
    let col = arg_usize1(call, 0)? - 1;
    let sep = unescape(&arg_string(call, 1)?);
    let mut out = Vec::new();
    for row in &rec.rows {
        if col >= row.len() {
            out.push(row.clone());
            continue;
        }
        for part in row[col].split(&sep) {
            let mut new_row = row.clone();
            new_row[col] = part.to_string();
            out.push(new_row);
        }
    }
    rec.rows = out;
    Ok(rec)
}

fn rec_implode(mut rec: Records, call: &Call) -> Result<Records> {
    let key_col = arg_usize1(call, 0)? - 1;
    let val_col = arg_usize1(call, 1)? - 1;
    let join_sep = if call.args.len() >= 3 {
        unescape(&arg_string(call, 2)?)
    } else {
        " ".to_string()
    };
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in &rec.rows {
        if key_col >= row.len() || val_col >= row.len() {
            continue;
        }
        map.entry(row[key_col].clone())
            .or_default()
            .push(row[val_col].clone());
    }
    rec.rows = map
        .into_iter()
        .map(|(k, vals)| vec![k, vals.join(&join_sep)])
        .collect();
    Ok(rec)
}

fn rec_groupby(mut rec: Records, call: &Call) -> Result<Records> {
    let key_col = arg_usize1(call, 0)? - 1;
    let aggs: Result<Vec<Agg>> = call.args[1..].iter().map(parse_agg).collect();
    let aggs = aggs?;
    let mut groups: BTreeMap<String, Vec<&Vec<String>>> = BTreeMap::new();
    for row in &rec.rows {
        if key_col < row.len() {
            groups.entry(row[key_col].clone()).or_default().push(row);
        }
    }
    let mut out = Vec::new();
    for (key, rows) in groups {
        let mut row = vec![key];
        for agg in &aggs {
            row.push(eval_agg(agg, &rows)?);
        }
        out.push(row);
    }
    rec.rows = out;
    Ok(rec)
}

fn rec_reshape(mut rec: Records, call: &Call) -> Result<Records> {
    let mode = arg_string(call, 0)?;
    match mode.as_str() {
        "w2l" | "wide2long" => {
            if rec.rows.is_empty() {
                return Ok(rec);
            }
            let start = arg_usize1(call, 1)? - 1;
            let header = rec.rows[0].clone();
            let mut out = Vec::new();
            for row in rec.rows.iter().skip(1) {
                let keys = row[..start.min(row.len())].to_vec();
                for i in start..row.len() {
                    let mut new_row = keys.clone();
                    new_row.push(
                        header
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("c{}", i + 1)),
                    );
                    new_row.push(row[i].clone());
                    out.push(new_row);
                }
            }
            rec.rows = out;
            Ok(rec)
        }
        "l2w" | "long2wide" => {
            let key_col = arg_usize1(call, 1)? - 1;
            let val_col = arg_usize1(call, 2)? - 1;
            if rec.rows.is_empty() {
                return Ok(rec);
            }
            let mut cols_seen = BTreeSet::new();
            let mut row_order = Vec::new();
            let mut seen_row = BTreeSet::new();
            let mut map: BTreeMap<String, HashMap<String, String>> = BTreeMap::new();
            for row in &rec.rows {
                if row.len() <= val_col || row.len() <= key_col {
                    continue;
                }
                let row_key = row[0].clone();
                let col_name = row[key_col].clone();
                let value = row[val_col].clone();
                cols_seen.insert(col_name.clone());
                if seen_row.insert(row_key.clone()) {
                    row_order.push(row_key.clone());
                }
                map.entry(row_key).or_default().insert(col_name, value);
            }
            let cols: Vec<String> = cols_seen.into_iter().collect();
            let mut out = vec![{
                let mut h = vec!["key".to_string()];
                h.extend(cols.clone());
                h
            }];
            for rk in row_order {
                let mut row = vec![rk.clone()];
                let vals = map.get(&rk).cloned().unwrap_or_default();
                for c in &cols {
                    row.push(vals.get(c).cloned().unwrap_or_default());
                }
                out.push(row);
            }
            rec.rows = out;
            Ok(rec)
        }
        _ => bail!("reshape mode must be w2l or l2w"),
    }
}

fn rec_flatten_as_records(mut rec: Records, call: &Call) -> Result<Records> {
    if rec.rows.is_empty() {
        rec.rows.clear();
        return Ok(rec);
    }
    let template = if !call.args.is_empty() {
        Some(arg_string(call, 0)?)
    } else {
        None
    };
    if let Some(tpl) = template {
        if rec.rows.len() < 2 {
            rec.rows = vec![vec![tpl]];
            return Ok(rec);
        }
        let header = &rec.rows[0];
        let mut out = Vec::new();
        let re = Regex::new(r"\{([^}]+)\}")?;
        for row in rec.rows.iter().skip(1) {
            let rendered = re
                .replace_all(&tpl, |caps: &regex::Captures| {
                    let key = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                    match header.iter().position(|h| h == key) {
                        Some(idx) => row.get(idx).cloned().unwrap_or_default(),
                        None => String::new(),
                    }
                })
                .to_string();
            out.push(vec![rendered]);
        }
        rec.rows = out;
    } else if rec.rows.len() >= 2 {
        let header = rec.rows[0].clone();
        let mut out = Vec::new();
        for row in rec.rows.iter().skip(1) {
            for (i, cell) in row.iter().enumerate() {
                let key = header
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("c{}", i + 1));
                out.push(vec![format!("{}: {}", key, cell)]);
            }
        }
        rec.rows = out;
    }
    rec.cfg.ofs = "".to_string();
    Ok(rec)
}

fn grid_transpose(mut grid: Grid) -> Result<Grid> {
    let h = grid.cells.len();
    let w = grid.cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut out = vec![vec![String::new(); h]; w];
    for y in 0..h {
        for x in 0..grid.cells[y].len() {
            out[x][y] = grid.cells[y][x].clone();
        }
    }
    grid.cells = out;
    Ok(grid)
}

fn grid_rotate(mut grid: Grid, call: &Call) -> Result<Grid> {
    let dir = arg_string(call, 0)?;
    let h = grid.cells.len();
    let w = grid.cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut padded = vec![vec![String::new(); w]; h];
    for y in 0..h {
        for x in 0..grid.cells[y].len() {
            padded[y][x] = grid.cells[y][x].clone();
        }
    }
    grid.cells = match dir.as_str() {
        "r" | "right" => {
            let mut out = vec![vec![String::new(); h]; w];
            for y in 0..h {
                for x in 0..w {
                    out[x][h - 1 - y] = padded[y][x].clone();
                }
            }
            out
        }
        "l" | "left" => {
            let mut out = vec![vec![String::new(); h]; w];
            for y in 0..h {
                for x in 0..w {
                    out[w - 1 - x][y] = padded[y][x].clone();
                }
            }
            out
        }
        "180" => {
            let mut out = vec![vec![String::new(); w]; h];
            for y in 0..h {
                for x in 0..w {
                    out[h - 1 - y][w - 1 - x] = padded[y][x].clone();
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
                            let mut seen = 0usize;
                            while cy >= 0
                                && (cy as usize) < grid.cells.len()
                                && cx >= 0
                                && (cx as usize) < grid.cells[cy as usize].len()
                            {
                                let cur = &grid.cells[cy as usize][cx as usize];
                                if through_pat.is_match(cur) {
                                    seen += 1;
                                    cx += dx;
                                    cy += dy;
                                    continue;
                                }
                                if seen > 0 && cur == &to {
                                    marks.push((cx as usize, cy as usize));
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

fn eval_agg(agg: &Agg, rows: &[&Vec<String>]) -> Result<String> {
    match agg {
        Agg::Count => Ok(rows.len().to_string()),
        Agg::Sum(c) => {
            let mut s = 0f64;
            for row in rows {
                s += row
                    .get(*c)
                    .map(|v| parse_num(v))
                    .transpose()?
                    .unwrap_or(0.0);
            }
            Ok(trim_num(s))
        }
        Agg::Min(c) => {
            let vals: Result<Vec<f64>> = rows
                .iter()
                .map(|r| {
                    r.get(*c)
                        .map(|v| parse_num(v))
                        .transpose()
                        .map(|o| o.unwrap_or(f64::INFINITY))
                })
                .collect();
            Ok(trim_num(vals?.into_iter().fold(f64::INFINITY, f64::min)))
        }
        Agg::Max(c) => {
            let vals: Result<Vec<f64>> = rows
                .iter()
                .map(|r| {
                    r.get(*c)
                        .map(|v| parse_num(v))
                        .transpose()
                        .map(|o| o.unwrap_or(f64::NEG_INFINITY))
                })
                .collect();
            Ok(trim_num(
                vals?.into_iter().fold(f64::NEG_INFINITY, f64::max),
            ))
        }
        Agg::Avg(c) => {
            let mut s = 0f64;
            let mut n = 0f64;
            for row in rows {
                if let Some(v) = row.get(*c) {
                    s += parse_num(v)?;
                    n += 1.0;
                }
            }
            Ok(trim_num(if n == 0.0 { 0.0 } else { s / n }))
        }
    }
}

fn parse_agg(expr: &Expr) -> Result<Agg> {
    match expr {
        Expr::Call(c) => match c.name.as_str() {
            "sum" | "s" => Ok(Agg::Sum(arg_usize_expr(c, 0)? - 1)),
            "count" | "c" => Ok(Agg::Count),
            "min" | "mn" => Ok(Agg::Min(arg_usize_expr(c, 0)? - 1)),
            "max" | "mx" => Ok(Agg::Max(arg_usize_expr(c, 0)? - 1)),
            "avg" | "a" => Ok(Agg::Avg(arg_usize_expr(c, 0)? - 1)),
            other => bail!("unknown aggregator: {other}"),
        },
        _ => bail!("groupby aggregators must be function calls"),
    }
}

fn arg_string(call: &Call, idx: usize) -> Result<String> {
    call.args
        .get(idx)
        .ok_or_else(|| anyhow!("missing arg {} for {}", idx + 1, call.name))
        .and_then(expr_to_string)
}

fn expr_to_string(e: &Expr) -> Result<String> {
    match e {
        Expr::Str(s) => Ok(s.clone()),
        Expr::Num(n) => Ok(n.to_string()),
        Expr::Ident(s) => Ok(s.clone()),
        Expr::Call(_) => bail!("nested call cannot be converted to string here"),
    }
}

fn arg_usize1(call: &Call, idx: usize) -> Result<usize> {
    arg_usize_expr(call, idx)
}

fn arg_usize_expr(call: &Call, idx: usize) -> Result<usize> {
    match call.args.get(idx) {
        Some(Expr::Num(n)) if *n > 0 => Ok(*n as usize),
        Some(Expr::Str(s)) => s
            .parse::<usize>()
            .with_context(|| format!("expected integer arg for {}", call.name)),
        _ => bail!(
            "expected positive integer arg {} for {}",
            idx + 1,
            call.name
        ),
    }
}

fn parse_num(s: &str) -> Result<f64> {
    s.parse::<f64>()
        .with_context(|| format!("not a number: {s}"))
}

fn trim_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

fn expand_spec(spec: &str, len: usize) -> Result<Vec<usize>> {
    if let Some((a, b)) = spec.split_once(':') {
        let start = if a.is_empty() { 1 } else { a.parse::<usize>()? };
        let end = if b.is_empty() {
            len
        } else {
            b.parse::<usize>()?
        };
        if start == 0 || end == 0 {
            bail!("field positions are 1-based: {spec}");
        }
        Ok((start - 1..end.min(len)).collect())
    } else {
        let n = spec.parse::<usize>()?;
        if n == 0 {
            bail!("field positions are 1-based: {spec}");
        }
        Ok(vec![n - 1])
    }
}

fn unescape(s: &str) -> String {
    s.replace(r"\t", "\t")
        .replace(r"\n", "\n")
        .replace(r"\r", "\r")
}

fn split_keep_nonempty<'a>(input: &'a str, sep: &str) -> Vec<&'a str> {
    if sep == "\n" {
        input.lines().collect()
    } else {
        input.split(sep).filter(|s| !s.is_empty()).collect()
    }
}

fn parse_program(src: &str) -> Result<Vec<Statement>> {
    let stmts = split_top_level(src, ';');
    let mut out = Vec::new();
    for stmt in stmts {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        out.push(parse_statement(s)?);
    }
    Ok(out)
}

fn parse_statement(src: &str) -> Result<Statement> {
    let (receiver, rest) = if let Some(rest) = src.strip_prefix("r.") {
        (Receiver::Rec, rest)
    } else if let Some(rest) = src.strip_prefix("rec.") {
        (Receiver::Rec, rest)
    } else if let Some(rest) = src.strip_prefix("d.") {
        (Receiver::Grid, rest)
    } else if let Some(rest) = src.strip_prefix("grid.") {
        (Receiver::Grid, rest)
    } else {
        bail!("statement must start with r./rec. or d./grid.: {src}")
    };
    let parts = split_top_level(rest, '.');
    let calls: Result<Vec<Call>> = parts.into_iter().map(|p| parse_call(p.trim())).collect();
    Ok(Statement {
        receiver,
        calls: calls?,
    })
}

fn parse_call(src: &str) -> Result<Call> {
    let open = src
        .find('(')
        .ok_or_else(|| anyhow!("expected '(' in call: {src}"))?;
    let close = src
        .rfind(')')
        .ok_or_else(|| anyhow!("expected ')' in call: {src}"))?;
    let name = src[..open].trim().to_string();
    let inner = &src[open + 1..close];
    let args = if inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(inner, ',')
            .into_iter()
            .map(|a| parse_expr(a.trim()))
            .collect::<Result<Vec<_>>>()?
    };
    Ok(Call { name, args })
}

fn parse_expr(src: &str) -> Result<Expr> {
    let s = src.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return Ok(Expr::Str(
            s[1..s.len() - 1].replace(r#"\""#, '"'.to_string().as_str()),
        ));
    }
    if s.starts_with('(') && s.ends_with(')') {
        return parse_expr(&s[1..s.len() - 1]);
    }
    if s.contains('(') && s.ends_with(')') {
        return Ok(Expr::Call(parse_call(s)?));
    }
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Expr::Num(n));
    }
    Ok(Expr::Ident(s.to_string()))
}

fn split_top_level(src: &str, delim: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut cur = String::new();
    let mut prev = '\0';
    for ch in src.chars() {
        match ch {
            '"' if prev != '\\' => {
                in_str = !in_str;
                cur.push(ch);
            }
            '(' if !in_str => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_str => {
                depth -= 1;
                cur.push(ch);
            }
            _ if ch == delim && !in_str && depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
        prev = ch;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}
