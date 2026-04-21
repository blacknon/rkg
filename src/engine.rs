use anyhow::Result;

use crate::ast::{GridConfig, Pipeline, RecConfig, Receiver, Statement};
use crate::grid::{apply_grid_call, parse_grid, render_grid};
use crate::record::{apply_rec_call, parse_records, render_records};

pub fn eval_statement_with_configs(
    stmt: &Statement,
    input: &str,
    rec_cfg: &RecConfig,
    grid_cfg: &GridConfig,
) -> Result<String> {
    match stmt.receiver {
        Receiver::Rec => {
            let mut rec = parse_records(input, rec_cfg.clone())?;
            for call in &stmt.calls {
                rec = apply_rec_call(rec, call)?;
            }
            Ok(render_records(&rec))
        }
        Receiver::Grid => {
            let mut grid = parse_grid(input, grid_cfg.clone())?;
            for call in &stmt.calls {
                grid = apply_grid_call(grid, call)?;
            }
            Ok(render_grid(&grid))
        }
    }
}

pub fn eval_pipeline_with_configs(
    pipeline: &Pipeline,
    input: &str,
    rec_cfg: &RecConfig,
    grid_cfg: &GridConfig,
) -> Result<String> {
    let mut current = input.to_string();
    for stage in &pipeline.stages {
        current = eval_statement_with_configs(stage, &current, rec_cfg, grid_cfg)?;
    }
    Ok(current)
}
