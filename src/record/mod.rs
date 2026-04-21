use anyhow::{Context, Result};
use regex::Regex;

use crate::ast::{Call, RecConfig, Records};
use crate::util::{arg_string, split_keep_nonempty, unescape};

mod aggs;
mod ops;
mod reshape;

/// Parse the current input into record rows and fields using the active record
/// separators.
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

/// Character mode is a record-specific fast path used by `chars()` / `ch()`.
pub fn parse_records_as_chars(input: &str, cfg: RecConfig) -> Result<Records> {
    let rs = unescape(&cfg.rs);
    let mut rows = Vec::new();

    for raw in split_keep_nonempty(input, &rs) {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        rows.push(line.chars().map(|ch| ch.to_string()).collect());
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

/// Apply one DSL call to the current record value.
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
        "chars" | "ch" => ops::rec_chars(rec),
        "countif" | "ci" => ops::rec_countif(rec, call),
        "select" | "p" => ops::rec_select(rec, &call.args),
        "replace" | "sb" => ops::rec_replace(rec, call),
        "enum" | "n" => ops::rec_enum(rec, call),
        "explode" | "x" => ops::rec_explode(rec, call),
        "implode" | "i" => ops::rec_implode(rec, call),
        "groupby" | "g" => aggs::rec_groupby(rec, call),
        "reshape" | "sh" => reshape::rec_reshape(rec, call),
        "flatten" | "f" => reshape::rec_flatten_as_records(rec, call),
        other => anyhow::bail!("unknown rec method: {other}"),
    }
}
