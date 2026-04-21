use anyhow::{bail, Result};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::ast::{Call, Records};
use crate::util::{arg_string, arg_usize1};

/// `reshape` has two modes:
/// - `w2l`: treat the header row as column names and expand repeated values
/// - `l2w`: rebuild a wide table keyed by column 1 and a named attribute column
pub(super) fn rec_reshape(rec: Records, call: &Call) -> Result<Records> {
    let mode = arg_string(call, 0)?;
    match mode.as_str() {
        "w2l" | "wide2long" => reshape_wide_to_long(rec, call),
        "l2w" | "long2wide" => reshape_long_to_wide(rec, call),
        _ => bail!("reshape mode must be w2l or l2w"),
    }
}

/// `flatten` can either render with a template or emit one `key: value` row per cell.
pub(super) fn rec_flatten_as_records(mut rec: Records, call: &Call) -> Result<Records> {
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

fn reshape_wide_to_long(mut rec: Records, call: &Call) -> Result<Records> {
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

fn reshape_long_to_wide(mut rec: Records, call: &Call) -> Result<Records> {
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
