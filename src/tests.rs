use anyhow::Result;

use crate::ast::{GridConfig, Pipeline, RecConfig};
use crate::parser::parse_program;

fn run(expr: &str, input: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let rec_cfg = RecConfig::default();
    let grid_cfg = GridConfig::default();
    for pipeline in &stmts {
        outputs.push(crate::engine::eval_pipeline_with_configs(
            pipeline, input, &rec_cfg, &grid_cfg,
        )?);
    }
    Ok(outputs.last().cloned().unwrap_or_default())
}

fn run_without_expr(input: &str, rec_cfg: RecConfig, grid_cfg: GridConfig) -> Result<String> {
    let pipeline = Pipeline {
        stages: vec![crate::ast::Statement {
            receiver: crate::ast::Receiver::Rec,
            calls: Vec::new(),
        }],
    };
    crate::engine::eval_pipeline_with_configs(&pipeline, input, &rec_cfg, &grid_cfg)
}

fn assert_run(expr: &str, input: &str, expected: &str) {
    let out = run(expr, input).unwrap_or_else(|err| panic!("`{expr}` should succeed: {err}"));
    assert_eq!(out, expected);
}

fn run_with_fs(expr: &str, input: &str, fs: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let mut rec_cfg = RecConfig::default();
    rec_cfg.fs = fs.to_string();
    let grid_cfg = GridConfig::default();
    for pipeline in &stmts {
        outputs.push(crate::engine::eval_pipeline_with_configs(
            pipeline, input, &rec_cfg, &grid_cfg,
        )?);
    }
    Ok(outputs.last().cloned().unwrap_or_default())
}

fn run_with_configs(
    expr: &str,
    input: &str,
    rec_cfg: RecConfig,
    grid_cfg: GridConfig,
) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    for pipeline in &stmts {
        outputs.push(crate::engine::eval_pipeline_with_configs(
            pipeline, input, &rec_cfg, &grid_cfg,
        )?);
    }
    Ok(outputs.last().cloned().unwrap_or_default())
}

#[test]
fn readme_record_example_works() {
    assert_run(
        r#"r.fs(",").x(2,";").g(1,s(2)).ofs(",")"#,
        "A,10;20;30\nB,7;8\n",
        "A,60\nB,15\n",
    );
}

#[test]
fn readme_grid_example_works() {
    assert_run(r#"d.t().rt("r")"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
}

#[test]
fn statement_reset_uses_original_stdin() {
    let stmts = parse_program(r#"r.x(2,",").g(1,s(2)); r.n(1)"#).expect("program should parse");
    let input = "A 10,20\nB 7,8\n";
    let rec_cfg = RecConfig::default();
    let grid_cfg = GridConfig::default();
    let outputs = stmts
        .iter()
        .map(|pipeline| {
            crate::engine::eval_pipeline_with_configs(pipeline, input, &rec_cfg, &grid_cfg)
        })
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
fn pipe_passes_record_output_into_grid_input() {
    assert_run(r#"r.ofs=| | d.t"#, "A 10\nB 20\n", "AB\n||\n12\n00\n");
}

#[test]
fn pipe_does_not_conflict_with_bare_pipe_argument() {
    assert_run(r#"r.p:1,2.ofs=|"#, "A 10\nB 20\n", "A|10\nB|20\n");
}

#[test]
fn pattern_mark_marks_through_cells() {
    assert_run(
        r#"d.m("X","O","X","*")"#,
        ".....\n.XOOX\n.....\n",
        ".....\n.X**X\n.....\n",
    );
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

#[test]
fn record_separator_option_applies_before_record_dsl() {
    let mut rec_cfg = RecConfig::default();
    rec_cfg.rs = "|".to_string();
    let out = run_with_configs(r#"r.p(1,2)"#, "A 10|B 20|", rec_cfg, GridConfig::default())
        .expect("record separator should succeed");
    assert_eq!(out, "A 10\nB 20\n");
}

#[test]
fn output_separators_option_apply_before_record_dsl() {
    let mut rec_cfg = RecConfig::default();
    rec_cfg.ofs = ",".to_string();
    rec_cfg.ors = "|".to_string();
    let out = run_with_configs(
        r#"r.p(1,2)"#,
        "A 10\nB 20\n",
        rec_cfg,
        GridConfig::default(),
    )
    .expect("output separators should succeed");
    assert_eq!(out, "A,10|B,20|");
}

#[test]
fn output_separators_option_apply_before_grid_dsl() {
    let mut grid_cfg = GridConfig::default();
    grid_cfg.ofs = "|".to_string();
    grid_cfg.ors = "---\n".to_string();
    let out = run_with_configs(r#"d.t"#, "ab\ncd\n", RecConfig::default(), grid_cfg)
        .expect("grid output separators should succeed");
    assert_eq!(out, "a|c---\nb|d---\n");
}

#[test]
fn option_only_run_defaults_to_record_passthrough() {
    let mut rec_cfg = RecConfig::default();
    rec_cfg.ofs = ",".to_string();
    let out = run_without_expr("A 10\nB 20\n", rec_cfg, GridConfig::default())
        .expect("option-only record passthrough should succeed");
    assert_eq!(out, "A,10\nB,20\n");
}

#[test]
fn colon_call_syntax_works() {
    assert_run(
        r#"r.p:1,3.ofs:|"#,
        "A 10 tokyo\nB 20 osaka\n",
        "A|tokyo\nB|osaka\n",
    );
}

#[test]
fn colon_call_syntax_supports_nested_calls() {
    assert_run(r#"r.g:1,s:2"#, "A 10\nA 20\nB 7\nB 8\n", "A 30\nB 15\n");
}

#[test]
fn equals_call_syntax_works_for_settings() {
    assert_run(
        r#"r.ofs=|"#,
        "A 10 tokyo\nB 20 osaka\n",
        "A|10|tokyo\nB|20|osaka\n",
    );
}

#[test]
fn equals_and_colon_call_syntax_can_mix() {
    assert_run(
        r#"r.p:1,3.ofs=|"#,
        "A 10 tokyo\nB 20 osaka\n",
        "A|tokyo\nB|osaka\n",
    );
}

#[test]
fn bare_zero_arg_call_syntax_works() {
    assert_run(r#"d.t.rt:r"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
}

#[test]
fn shorthand_record_syntax_matches_classic_syntax() {
    let input = "A,10;tokyo\nB:20;osaka\n";
    let classic = run(r#"r.p(1,2,3).ofs("|")"#, input).expect("classic syntax should succeed");
    let shorthand = run(r#"r.p:1,2,3.ofs=|"#, input).expect("shorthand syntax should succeed");
    assert_eq!(shorthand, classic);
}

#[test]
fn shorthand_grid_syntax_matches_classic_syntax() {
    let input = "abc\ndef\nghi\n";
    let classic = run(r#"d.t().rt("r")"#, input).expect("classic syntax should succeed");
    let shorthand = run(r#"d.t.rt:r"#, input).expect("shorthand syntax should succeed");
    assert_eq!(shorthand, classic);
}

#[test]
fn median_aggregator_returns_middle_value_or_average_of_middle_pair() {
    assert_run(
        r#"r.g(1,med(2))"#,
        "A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n",
        "A 15\nB 9.5\nC 6\n",
    );
}

#[test]
fn median_aggregator_shorthand_works() {
    assert_run(r#"r.g:1,med:2"#, "A 10\nA 30\nA 20\n", "A 20\n");
}
