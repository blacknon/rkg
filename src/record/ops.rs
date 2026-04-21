use anyhow::{Context, Result};
use regex::Regex;
use std::collections::BTreeMap;

use crate::ast::{Call, Expr, Records};
use crate::util::{arg_string, arg_usize1, expr_to_string, unescape};

/// Re-parse each rendered row as a sequence of single-character fields.
pub(super) fn rec_chars(mut rec: Records) -> Result<Records> {
    rec.rows = rec
        .rows
        .iter()
        .map(|row| row.join(&unescape(&rec.cfg.ofs)))
        .map(|line| line.chars().map(|ch| ch.to_string()).collect())
        .collect();
    rec.cfg.ofs = "".to_string();
    Ok(rec)
}

pub(super) fn rec_countif(mut rec: Records, call: &Call) -> Result<Records> {
    let pat = arg_string(call, 0)?;
    let re = Regex::new(&pat).with_context(|| format!("invalid regex: {pat}"))?;
    rec.rows = rec
        .rows
        .iter()
        .map(|row| {
            let count = row.iter().filter(|cell| re.is_match(cell)).count();
            vec![count.to_string()]
        })
        .collect();
    Ok(rec)
}

pub(super) fn rec_select(mut rec: Records, args: &[Expr]) -> Result<Records> {
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

pub(super) fn rec_replace(mut rec: Records, call: &Call) -> Result<Records> {
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

pub(super) fn rec_enum(mut rec: Records, call: &Call) -> Result<Records> {
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

pub(super) fn rec_explode(mut rec: Records, call: &Call) -> Result<Records> {
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

pub(super) fn rec_implode(mut rec: Records, call: &Call) -> Result<Records> {
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

/// Field specs support `N` and `start:end` with 1-based indexing.
fn expand_spec(spec: &str, len: usize) -> Result<Vec<usize>> {
    if let Some((a, b)) = spec.split_once(':') {
        let start = if a.is_empty() { 1 } else { a.parse::<usize>()? };
        let end = if b.is_empty() {
            len
        } else {
            b.parse::<usize>()?
        };
        if start == 0 || end == 0 {
            anyhow::bail!("field positions are 1-based: {spec}");
        }
        Ok((start - 1..end.min(len)).collect())
    } else {
        let n = spec.parse::<usize>()?;
        if n == 0 {
            anyhow::bail!("field positions are 1-based: {spec}");
        }
        Ok(vec![n - 1])
    }
}
