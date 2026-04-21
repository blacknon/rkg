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
    assert_run(r#"g.t().rt("r")"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
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
    assert_run(r#"r.ofs=| | g.t"#, "A 10\nB 20\n", "AB\n||\n12\n00\n");
}

#[test]
fn pipe_does_not_conflict_with_bare_pipe_argument() {
    assert_run(r#"r.p:1,2.ofs=|"#, "A 10\nB 20\n", "A|10\nB|20\n");
}

#[test]
fn pattern_mark_marks_through_cells() {
    assert_run(
        r#"g.m("X","O","X","*")"#,
        ".....\n.XOOX\n.....\n",
        ".....\n.X**X\n.....\n",
    );
}

#[test]
fn mark_accepts_orth_ray_name() {
    assert_run(
        r#"g.m("K","orth","*")"#,
        ".....\n..K..\n.....\n",
        "..*..\n**K**\n..*..\n",
    );
}

#[test]
fn mark_accepts_diag_ray_name() {
    assert_run(
        r#"g.m("K","diag","*")"#,
        ".....\n..K..\n.....\n",
        ".*.*.\n..K..\n.*.*.\n",
    );
}

#[test]
fn mark_can_pick_origin_by_value() {
    assert_run(
        r#"g.m(pick("K"),"orth","*")"#,
        ".......\n.......\n...K...\n.......\n.......\n",
        "...*...\n...*...\n***K***\n...*...\n...*...\n",
    );
}

#[test]
fn mark_can_pick_nth_origin_by_value() {
    assert_run(
        r#"g.m(pick("K",2),"diag","*")"#,
        "K....\n.....\n..K..\n.....\n....K\n",
        "*...*\n.*.*.\n..K..\n.*.*.\n*...*\n",
    );
}

#[test]
fn mark_can_use_p_alias_for_pick() {
    assert_run(
        r#"g.m(p("K"),"orth","*")"#,
        ".......\n.......\n...K...\n.......\n.......\n",
        "...*...\n...*...\n***K***\n...*...\n...*...\n",
    );
}

#[test]
fn mark_can_use_p_alias_for_nth_pick() {
    assert_run(
        r#"g.m(p("K",2),"diag","*")"#,
        "K....\n.....\n..K..\n.....\n....K\n",
        "*...*\n.*.*.\n..K..\n.*.*.\n*...*\n",
    );
}

#[test]
fn get_can_read_by_coordinates() {
    assert_run(r#"g.get(3,2)"#, "abc\ndef\nghi\n", "f\n");
}

#[test]
fn get_can_read_by_pick_origin() {
    assert_run(r#"g.get(p("K"))"#, ".....\n..K..\n.....\n", "K\n");
}

#[test]
fn set_can_write_by_coordinates() {
    assert_run(r#"g.set(3,2,"X")"#, "abc\ndef\nghi\n", "abc\ndeX\nghi\n");
}

#[test]
fn set_can_write_by_pick_origin() {
    assert_run(r#"g.set(p("K"),".")"#, ".....\n..K..\n.....\n", ".....\n.....\n.....\n");
}

#[test]
fn reverse_can_flip_horizontal() {
    assert_run(r#"g.rev("h")"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
}

#[test]
fn reverse_can_flip_vertical() {
    assert_run(r#"g.rev("v")"#, "abc\ndef\nghi\n", "ghi\ndef\nabc\n");
}

#[test]
fn reverse_can_pad_before_horizontal_flip() {
    assert_run(r#"g.rev("h",pad("."))"#, "ab\ncde\n", ".ba\nedc\n");
}

#[test]
fn reverse_can_flip_both_axes() {
    assert_run(r#"g.rev("hv")"#, "abc\ndef\n", "fed\ncba\n");
}

#[test]
fn line_can_write_right_from_coordinates() {
    assert_run(
        r#"g.line(2,2,"right","A","B","C")"#,
        ".....\n.....\n.....\n",
        ".....\n.ABC.\n.....\n",
    );
}

#[test]
fn line_can_write_vertical_centered_from_pick() {
    assert_run(
        r#"g.line(p("K"),"vert","A","B","C")"#,
        ".....\n.....\n..K..\n.....\n.....\n",
        ".....\n..A..\n..B..\n..C..\n.....\n",
    );
}

#[test]
fn line_can_write_horizontal_centered() {
    assert_run(
        r#"g.line(3,2,"horiz","A","B","C")"#,
        ".....\n.....\n.....\n",
        ".....\n.ABC.\n.....\n",
    );
}

#[test]
fn line_can_write_diagonal_centered() {
    assert_run(
        r#"g.line(3,3,"diag_dr","A","B","C")"#,
        ".....\n.....\n.....\n.....\n.....\n",
        ".....\n.A...\n..B..\n...C.\n.....\n",
    );
}

#[test]
fn line_direction_aliases_work() {
    assert_run(
        r#"g.line(2,2,"r","A","B","C")"#,
        ".....\n.....\n.....\n",
        ".....\n.ABC.\n.....\n",
    );
    assert_run(
        r#"g.line(3,3,"v","A","B","C")"#,
        ".....\n.....\n.....\n.....\n.....\n",
        ".....\n..A..\n..B..\n..C..\n.....\n",
    );
}

#[test]
fn line_can_wrap_rows() {
    assert_run(
        r#"g.line(4,1,"r","A","B","C","D",wrap("row"))"#,
        ".....\n.....\n.....\n",
        "...AB\nCD...\n.....\n",
    );
}

#[test]
fn line_can_wrap_columns() {
    assert_run(
        r#"g.line(2,3,"d","1","2","3","4",wrap("col"))"#,
        "....\n....\n....\n",
        "..2.\n..3.\n.14.\n",
    );
}

#[test]
fn line_can_wrap_diagonal() {
    assert_run(
        r#"g.line(3,3,"dr","A","B","C","D",wrap("diag_dr"))"#,
        ".....\n.....\n.....\n.....\n.....\n",
        ".D...\n.....\n..A..\n...B.\n....C\n",
    );
}

#[test]
fn line_can_fill_upper_right_diagonals() {
    assert_run(
        r#"g.line(1,1,"fur","A","B","C","D","E","F","G","H","I",skip(1))"#,
        ".....\n.....\n.....\n.....\n.....\n",
        ".BEI.\nADH..\nCG...\nF....\n.....\n",
    );
}

#[test]
fn line_fill_mode_can_shift_start_with_skip() {
    assert_run(
        r#"g.line(1,1,"fur","A","B","C","D","E","F","G",skip(3))"#,
        ".....\n.....\n.....\n.....\n.....\n",
        "..CG.\n.BF..\nAE...\nD....\n.....\n",
    );
}

#[test]
fn mark_can_run_line_mode() {
    assert_run(
        r#"g.m(p("K"),"line","r","A","B","C")"#,
        ".....\n..K..\n.....\n",
        ".....\n..ABC\n.....\n",
    );
}

#[test]
fn mark_can_run_wrapped_line_mode() {
    assert_run(
        r#"g.m(p("K"),"line","r","A","B","C","D",wrap("row"))"#,
        ".....\n...K.\n.....\n",
        ".....\n...AB\nCD...\n",
    );
}

#[test]
fn mark_can_run_fill_line_mode() {
    assert_run(
        r#"g.m(p("K"),"line","fur","A","B","C","D",skip(1))"#,
        "K....\n.....\n.....\n.....\n",
        "KB...\nAD...\nC....\n.....\n",
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
    let out = run_with_configs(r#"g.t"#, "ab\ncd\n", RecConfig::default(), grid_cfg)
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
fn shorthand_outer_call_can_wrap_nested_paren_call() {
    assert_run(
        "g.m:p(\"K\",2),\"diag\",\"*\"",
        "K....\n.....\n..K..\n.....\n....K\n",
        "*...*\n.*.*.\n..K..\n.*.*.\n*...*\n",
    );
}

#[test]
fn shorthand_get_and_set_work() {
    assert_run(r#"g.get:3,2"#, "abc\ndef\nghi\n", "f\n");
    assert_run(r#"g.set:3,2,X"#, "abc\ndef\nghi\n", "abc\ndeX\nghi\n");
    assert_run(r#"g.rv:h"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
    assert_run(r#"g.rv:h,pad:".""#, "ab\ncde\n", ".ba\nedc\n");
}

#[test]
fn shorthand_line_works() {
    assert_run(
        r#"g.ln:2,2,r,A,B,C"#,
        ".....\n.....\n.....\n",
        ".....\n.ABC.\n.....\n",
    );
    assert_run(
        r#"g.ln:4,1,r,A,B,C,D,wrap:row"#,
        ".....\n.....\n.....\n",
        "...AB\nCD...\n.....\n",
    );
    assert_run(
        r#"g.ln:1,1,fur,A,B,C,D,E,F,G,H,I,skip:1"#,
        ".....\n.....\n.....\n.....\n.....\n",
        ".BEI.\nADH..\nCG...\nF....\n.....\n",
    );
}

#[test]
fn shorthand_mark_line_mode_works() {
    assert_run(
        r#"g.m:p("K"),line,r,A,B"#,
        ".....\n..K..\n.....\n",
        ".....\n..AB.\n.....\n",
    );
    assert_run(
        r#"g.m:p("K"),line,r,A,B,C,D,wrap:row"#,
        ".....\n...K.\n.....\n",
        ".....\n...AB\nCD...\n",
    );
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
    assert_run(r#"g.t.rt:r"#, "abc\ndef\nghi\n", "cba\nfed\nihg\n");
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
    let classic = run(r#"g.t().rt("r")"#, input).expect("classic syntax should succeed");
    let shorthand = run(r#"g.t.rt:r"#, input).expect("shorthand syntax should succeed");
    assert_eq!(shorthand, classic);
}

#[test]
fn grid_receiver_short_alias_g_is_accepted() {
    assert_run(r#"g.t"#, "ab\ncd\n", "ac\nbd\n");
}

#[test]
fn legacy_grid_receiver_d_is_rejected() {
    let err = run(r#"d.t"#, "ab\ncd\n").expect_err("legacy d receiver should fail");
    assert!(
        err.to_string()
            .contains("statement must start with r./rec. or g./grid."),
        "unexpected error: {err}"
    );
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
