use anyhow::{bail, Result};
use regex::Regex;

use crate::ast::{Address, AddressRange, GridConfig, Pipeline, RecConfig, Receiver, Source, Statement};
use crate::grid::{apply_grid_call, parse_grid, render_grid};
use crate::record::{apply_rec_call, parse_records, parse_records_as_chars, render_records};
use crate::util::{split_keep_nonempty, unescape};

pub fn eval_statement_with_configs(
    stmt: &Statement,
    input: &str,
    stdin_input: &str,
    rec_cfg: &RecConfig,
    grid_cfg: &GridConfig,
) -> Result<String> {
    let source_input = match &stmt.source {
        Source::Current => input,
        Source::Stdin => stdin_input,
        Source::Prev => input,
        Source::Named(name) => bail!("named source is not supported yet: {name}"),
    };

    match stmt.receiver {
        Receiver::Rec => {
            let input = if let Some(address) = &stmt.address {
                filter_records_by_address(source_input, &rec_cfg.rs, address)?
            } else {
                source_input.to_string()
            };
            let use_char_parse = matches!(
                stmt.calls.first().map(|call| call.name.as_str()),
                Some("chars" | "ch")
            );
            let mut rec = if use_char_parse {
                parse_records_as_chars(&input, rec_cfg.clone())?
            } else {
                parse_records(&input, rec_cfg.clone())?
            };
            let calls = if use_char_parse {
                &stmt.calls[1..]
            } else {
                &stmt.calls[..]
            };
            for call in calls {
                rec = apply_rec_call(rec, call)?;
            }
            Ok(render_records(&rec))
        }
        Receiver::Grid => {
            if stmt.address.is_some() {
                bail!("addresses are currently only supported for record statements");
            }
            let mut grid = parse_grid(source_input, grid_cfg.clone())?;
            for call in &stmt.calls {
                grid = apply_grid_call(grid, call)?;
            }
            Ok(render_grid(&grid))
        }
    }
}

fn filter_records_by_address(input: &str, rs: &str, address: &AddressRange) -> Result<String> {
    let rs = unescape(rs);
    let records = split_keep_nonempty(input, &rs)
        .into_iter()
        .map(|raw| raw.trim_end_matches('\r').to_string())
        .collect::<Vec<_>>();
    let selected = select_record_indexes(&records, address)?;
    let mut out = String::new();
    for idx in selected {
        if let Some(record) = records.get(idx) {
            out.push_str(record);
            out.push_str(&rs);
        }
    }
    Ok(out)
}

fn select_record_indexes(records: &[String], address: &AddressRange) -> Result<Vec<usize>> {
    if records.is_empty() {
        return Ok(Vec::new());
    }

    if address.end.is_none() {
        return select_single_address(records, &address.start);
    }

    let start = resolve_address(records, &address.start, 0)?;
    let end = resolve_address(records, address.end.as_ref().expect("checked above"), start)?;

    if start > end {
        return Ok(Vec::new());
    }

    Ok((start..=end).collect())
}

fn select_single_address(records: &[String], address: &Address) -> Result<Vec<usize>> {
    match address {
        Address::Regex(pat) => {
            let re = Regex::new(pat)?;
            Ok(records
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| re.is_match(row).then_some(idx))
                .collect())
        }
        _ => Ok(vec![resolve_address(records, address, 0)?]),
    }
}

fn resolve_address(records: &[String], address: &Address, min_index: usize) -> Result<usize> {
    match address {
        Address::Line(n) => {
            if *n == 0 {
                bail!("address positions are 1-based");
            }
            Ok(n.saturating_sub(1).min(records.len().saturating_sub(1)))
        }
        Address::Last => Ok(records.len().saturating_sub(1)),
        Address::Regex(pat) => {
            let re = Regex::new(pat)?;
            records
                .iter()
                .enumerate()
                .skip(min_index)
                .find_map(|(idx, row)| re.is_match(row).then_some(idx))
                .ok_or_else(|| anyhow::anyhow!("address regex did not match any record: /{pat}/"))
        }
    }
}

pub fn eval_pipeline_with_configs(
    pipeline: &Pipeline,
    input: &str,
    stdin_input: &str,
    rec_cfg: &RecConfig,
    grid_cfg: &GridConfig,
) -> Result<String> {
    let mut current = input.to_string();
    for stage in &pipeline.stages {
        current = eval_statement_with_configs(stage, &current, stdin_input, rec_cfg, grid_cfg)?;
    }
    Ok(current)
}
