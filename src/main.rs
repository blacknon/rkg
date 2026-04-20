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

use crate::ast::{GridConfig, RecConfig};

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

    /// DSL program, e.g. r.fs(",").x(2,";").g(1,s(2)); d.t().rt("r")
    expr: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let stmts = parser::parse_program(&cli.expr)?;
    if stmts.is_empty() {
        bail!("empty program");
    }

    let mut rec_cfg = RecConfig::default();
    if let Some(field_separator) = cli.field_separator {
        rec_cfg.fs = field_separator;
    }
    let grid_cfg = GridConfig::default();

    let outputs = stmts
        .iter()
        .map(|stmt| engine::eval_statement_with_configs(stmt, &input, &rec_cfg, &grid_cfg))
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
