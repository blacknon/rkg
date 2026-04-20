use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::ast::{Agg, Call, Expr, RecConfig, Records};
use crate::util::{
    arg_string, arg_usize1, arg_usize_expr, expr_to_string, parse_num, split_keep_nonempty,
    trim_num, unescape,
};

pub fn parse_records(input: &str, cfg: RecConfig) -> Result<Records> {
    let rs = unescape(&cfg.rs);
    let fs_re = Regex::new(&cfg.fs).with_context(|| format!("invalid FS regex: {}", cfg.fs))?;
    let mut rows = Vec::new();

    for raw in split_keep_nonempty(input, &rs) {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }

        let cols = if cfg.fs == r"\s+" {
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

pub fn render_records(rec: &Records) -> String {
    let mut out = String::new();
    let ors = unescape(&rec.cfg.ors);
    let ofs = unescape(&rec.cfg.ofs);
    for row in &rec.rows {
        out.push_str(&row.join(&ofs));
        out.push_str(&ors);
    }
    out
}

pub fn apply_rec_call(mut rec: Records, call: &Call) -> Result<Records> {
    match call.name.as_str() {
        "fs" => {
            rec.cfg.fs = arg_string(call, 0)?;
            parse_records(&render_records(&rec), rec.cfg.clone())
        }
        "rs" => {
            rec.cfg.rs = arg_string(call, 0)?;
            parse_records(&render_records(&rec), rec.cfg.clone())
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

fn rec_select(mut rec: Records, args: &[Expr]) -> Result<Records> {
    let specs = args
        .iter()
        .map(expr_to_string)
        .collect::<Result<Vec<_>>>()?;
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
        .map(|(key, vals)| vec![key, vals.join(&join_sep)])
        .collect();
    Ok(rec)
}

fn rec_groupby(mut rec: Records, call: &Call) -> Result<Records> {
    let key_col = arg_usize1(call, 0)? - 1;
    let aggs = call.args[1..]
        .iter()
        .map(parse_agg)
        .collect::<Result<Vec<_>>>()?;
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

            let cols = cols_seen.into_iter().collect::<Vec<_>>();
            let mut out = vec![{
                let mut header = vec!["key".to_string()];
                header.extend(cols.clone());
                header
            }];

            for row_key in row_order {
                let mut row = vec![row_key.clone()];
                let vals = map.get(&row_key).cloned().unwrap_or_default();
                for col in &cols {
                    row.push(vals.get(col).cloned().unwrap_or_default());
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
        let re = Regex::new(r"\{([^}]+)\}")?;
        let mut out = Vec::new();

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

fn parse_agg(expr: &Expr) -> Result<Agg> {
    match expr {
        Expr::Call(call) => match call.name.as_str() {
            "sum" | "s" => Ok(Agg::Sum(arg_usize_expr(call, 0)? - 1)),
            "count" | "c" => Ok(Agg::Count),
            "min" | "mn" => Ok(Agg::Min(arg_usize_expr(call, 0)? - 1)),
            "max" | "mx" => Ok(Agg::Max(arg_usize_expr(call, 0)? - 1)),
            "avg" | "a" => Ok(Agg::Avg(arg_usize_expr(call, 0)? - 1)),
            other => bail!("unknown aggregator: {other}"),
        },
        _ => bail!("groupby aggregators must be function calls"),
    }
}

fn eval_agg(agg: &Agg, rows: &[&Vec<String>]) -> Result<String> {
    match agg {
        Agg::Count => Ok(rows.len().to_string()),
        Agg::Sum(col) => {
            let mut sum = 0f64;
            for row in rows {
                sum += row
                    .get(*col)
                    .map(|v| parse_num(v))
                    .transpose()?
                    .unwrap_or(0.0);
            }
            Ok(trim_num(sum))
        }
        Agg::Min(col) => {
            let vals = rows
                .iter()
                .map(|row| {
                    row.get(*col)
                        .map(|v| parse_num(v))
                        .transpose()
                        .map(|v| v.unwrap_or(f64::INFINITY))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(trim_num(vals.into_iter().fold(f64::INFINITY, f64::min)))
        }
        Agg::Max(col) => {
            let vals = rows
                .iter()
                .map(|row| {
                    row.get(*col)
                        .map(|v| parse_num(v))
                        .transpose()
                        .map(|v| v.unwrap_or(f64::NEG_INFINITY))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(trim_num(vals.into_iter().fold(f64::NEG_INFINITY, f64::max)))
        }
        Agg::Avg(col) => {
            let mut sum = 0f64;
            let mut count = 0f64;
            for row in rows {
                if let Some(v) = row.get(*col) {
                    sum += parse_num(v)?;
                    count += 1.0;
                }
            }
            Ok(trim_num(if count == 0.0 { 0.0 } else { sum / count }))
        }
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
