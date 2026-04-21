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
    /// Print every statement result separated by ---
    #[arg(long)]
    print_all: bool,

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

    /// DSL program, e.g. r.fs(",").x(2,";").g(1,s(2)); d.t().rt("r")
    expr: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let stmts = if let Some(expr) = &cli.expr {
        let stmts = parser::parse_program(expr)?;
        if stmts.is_empty() {
            bail!("empty program");
        }
        stmts
    } else {
        vec![Pipeline {
            stages: vec![Statement {
                receiver: Receiver::Rec,
                calls: Vec::new(),
            }],
        }]
    };

    let mut rec_cfg = RecConfig::default();
    if let Some(field_separator) = cli.field_separator {
        rec_cfg.fs = field_separator;
    }
    if let Some(record_separator) = &cli.record_separator {
        rec_cfg.rs = record_separator.clone();
    }
    if let Some(output_field_separator) = &cli.output_field_separator {
        rec_cfg.ofs = output_field_separator.clone();
    }
    if let Some(output_record_separator) = &cli.output_record_separator {
        rec_cfg.ors = output_record_separator.clone();
    }

    let mut grid_cfg = GridConfig::default();
    if let Some(record_separator) = &cli.record_separator {
        grid_cfg.rs = record_separator.clone();
    }
    if let Some(output_field_separator) = &cli.output_field_separator {
        grid_cfg.ofs = output_field_separator.clone();
    }
    if let Some(output_record_separator) = &cli.output_record_separator {
        grid_cfg.ors = output_record_separator.clone();
    }

    let outputs = stmts
        .iter()
        .map(|pipeline| engine::eval_pipeline_with_configs(pipeline, &input, &rec_cfg, &grid_cfg))
        .collect::<Result<Vec<_>>>()?;

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
