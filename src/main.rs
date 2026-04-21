mod ast;
mod engine;
mod grid;
mod parser;
mod record;
mod util;

#[cfg(test)]
mod tests;

use anyhow::{bail, Result};
use clap::Parser;
use std::io::{self, Read};

use crate::ast::{GridConfig, Pipeline, RecConfig, Receiver, Statement};

#[derive(Parser, Debug)]
#[command(name = "rkg")]
#[command(about = "Record/grid DSL processor")]
struct Cli {
    /// Input field separator regex for record mode, like awk -F
    #[arg(short = 'F', long = "field-separator")]
    field_separator: Option<String>,

    /// Input record separator for both record and grid mode
    #[arg(short = 'R', long = "record-separator")]
    record_separator: Option<String>,

    /// Output field separator for both record and grid mode
    #[arg(short = 'O', long = "output-field-separator")]
    output_field_separator: Option<String>,

    /// Output record separator for both record and grid mode
    #[arg(short = 'N', long = "output-record-separator")]
    output_record_separator: Option<String>,

    /// DSL program, e.g. r.fs(",").x(2,";").g(1,s(2)); g.t().rt("r")
    expr: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let stmts = parse_or_default_program(cli.expr.as_deref())?;
    let rec_cfg = build_rec_config(&cli);
    let grid_cfg = build_grid_config(&cli);

    let mut current = input.clone();
    let mut outputs = Vec::new();
    for pipeline in &stmts {
        current =
            engine::eval_pipeline_with_configs(pipeline, &current, &input, &rec_cfg, &grid_cfg)?;
        outputs.push(current.clone());
    }

    for (i, out) in outputs.iter().enumerate() {
        if i > 0 {
            println!("---");
        }
        print!("{}", out);
    }

    Ok(())
}

fn parse_or_default_program(expr: Option<&str>) -> Result<Vec<Pipeline>> {
    if let Some(expr) = expr {
        let stmts = parser::parse_program(expr)?;
        if stmts.is_empty() {
            bail!("empty program");
        }
        Ok(stmts)
    } else {
        Ok(vec![default_pipeline()])
    }
}

fn default_pipeline() -> Pipeline {
    Pipeline {
        stages: vec![Statement {
            source: crate::ast::Source::Current,
            address: None,
            receiver: Receiver::Rec,
            calls: Vec::new(),
        }],
    }
}

/// Record mode has its own input field separator, plus the shared row/output
/// separators that are also available in grid mode.
fn build_rec_config(cli: &Cli) -> RecConfig {
    let mut cfg = RecConfig::default();
    if let Some(field_separator) = &cli.field_separator {
        cfg.fs = field_separator.clone();
    }
    if let Some(record_separator) = &cli.record_separator {
        cfg.rs = record_separator.clone();
    }
    if let Some(output_field_separator) = &cli.output_field_separator {
        cfg.ofs = output_field_separator.clone();
    }
    if let Some(output_record_separator) = &cli.output_record_separator {
        cfg.ors = output_record_separator.clone();
    }
    cfg
}

fn build_grid_config(cli: &Cli) -> GridConfig {
    let mut cfg = GridConfig::default();
    if let Some(record_separator) = &cli.record_separator {
        cfg.rs = record_separator.clone();
    }
    if let Some(output_field_separator) = &cli.output_field_separator {
        cfg.ofs = output_field_separator.clone();
    }
    if let Some(output_record_separator) = &cli.output_record_separator {
        cfg.ors = output_record_separator.clone();
    }
    cfg
}
