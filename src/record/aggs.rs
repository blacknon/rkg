use anyhow::{bail, Result};
use std::collections::BTreeMap;

use crate::ast::{Agg, Call, Expr, Records};
use crate::util::{arg_usize1, arg_usize_expr, parse_num, trim_num};

pub(super) fn rec_groupby(mut rec: Records, call: &Call) -> Result<Records> {
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

fn parse_agg(expr: &Expr) -> Result<Agg> {
    match expr {
        Expr::Call(call) => match call.name.as_str() {
            "sum" | "s" => Ok(Agg::Sum(arg_usize_expr(call, 0)? - 1)),
            "count" | "c" => Ok(Agg::Count),
            "min" | "mn" => Ok(Agg::Min(arg_usize_expr(call, 0)? - 1)),
            "max" | "mx" => Ok(Agg::Max(arg_usize_expr(call, 0)? - 1)),
            "avg" | "a" => Ok(Agg::Avg(arg_usize_expr(call, 0)? - 1)),
            "median" | "med" => Ok(Agg::Median(arg_usize_expr(call, 0)? - 1)),
            other => bail!("unknown aggregator: {other}"),
        },
        _ => bail!("groupby aggregators must be function calls"),
    }
}

/// Evaluate one aggregate over all rows in the current group.
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
        Agg::Median(col) => {
            let mut vals = Vec::new();
            for row in rows {
                if let Some(v) = row.get(*col) {
                    vals.push(parse_num(v)?);
                }
            }
            if vals.is_empty() {
                return Ok("0".to_string());
            }
            vals.sort_by(f64::total_cmp);
            let mid = vals.len() / 2;
            let median = if vals.len() % 2 == 0 {
                (vals[mid - 1] + vals[mid]) / 2.0
            } else {
                vals[mid]
            };
            Ok(trim_num(median))
        }
    }
}
