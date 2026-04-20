use anyhow::Result;

use crate::ast::{GridConfig, RecConfig};
use crate::parser::parse_program;

fn run(expr: &str, input: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let rec_cfg = RecConfig::default();
    let grid_cfg = GridConfig::default();
    for stmt in &stmts {
        outputs.push(crate::engine::eval_statement_with_configs(
            stmt, input, &rec_cfg, &grid_cfg,
        )?);
    }
    Ok(outputs.last().cloned().unwrap_or_default())
}

fn run_with_fs(expr: &str, input: &str, fs: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let mut rec_cfg = RecConfig::default();
    rec_cfg.fs = fs.to_string();
    let grid_cfg = GridConfig::default();
    for stmt in &stmts {
        outputs.push(crate::engine::eval_statement_with_configs(
            stmt, input, &rec_cfg, &grid_cfg,
        )?);
    }
    Ok(outputs.last().cloned().unwrap_or_default())
}

#[test]
fn readme_record_example_works() {
    let out = run(
        r#"r.fs(",").x(2,";").g(1,s(2)).ofs(",")"#,
        "A,10;20;30\nB,7;8\n",
    )
    .expect("record example should succeed");
    assert_eq!(out, "A,60\nB,15\n");
}

#[test]
fn readme_grid_example_works() {
    let out = run(r#"d.t().rt("r")"#, "abc\ndef\nghi\n").expect("grid example should succeed");
    assert_eq!(out, "cba\nfed\nihg\n");
}

#[test]
fn statement_reset_uses_original_stdin() {
    let stmts = parse_program(r#"r.x(2,",").g(1,s(2)); r.n(1)"#).expect("program should parse");
    let input = "A 10,20\nB 7,8\n";
    let rec_cfg = RecConfig::default();
    let grid_cfg = GridConfig::default();
    let outputs = stmts
        .iter()
        .map(|stmt| crate::engine::eval_statement_with_configs(stmt, input, &rec_cfg, &grid_cfg))
        .collect::<Result<Vec<_>>>()
        .expect("both statements should succeed");
    assert_eq!(
        outputs,
        vec![
            "A 30\nB 15\n".to_string(),
            "1 A 10,20\n2 B 7,8\n".to_string()
        ]
    );
}

#[test]
fn pattern_mark_marks_through_cells() {
    let out = run(r#"d.m("X","O","X","*")"#, ".....\n.XOOX\n.....\n")
        .expect("pattern mark should succeed");
    assert_eq!(out, ".....\n.X**X\n.....\n");
}

#[test]
fn field_separator_option_accepts_regex() {
    let out = run_with_fs(
        r#"r.p(1,2,3).ofs("|")"#,
        "A,10;tokyo\nB:20;osaka\n",
        r#"[,;:]"#,
    )
    .expect("field separator regex should succeed");
    assert_eq!(out, "A|10|tokyo\nB|20|osaka\n");
}
