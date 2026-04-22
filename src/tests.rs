use anyhow::Result;

use crate::ast::{GridConfig, Pipeline, RecConfig, Source};
use crate::parser::parse_program;

fn run(expr: &str, input: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let rec_cfg = RecConfig::default();
    let grid_cfg = GridConfig::default();
    let mut current = input.to_string();
    for pipeline in &stmts {
        current = crate::engine::eval_pipeline_with_configs(
            pipeline, &current, input, &rec_cfg, &grid_cfg,
        )?;
        outputs.push(current.clone());
    }
    Ok(join_outputs(&outputs))
}

fn run_without_expr(input: &str, rec_cfg: RecConfig, grid_cfg: GridConfig) -> Result<String> {
    let pipeline = Pipeline {
        stages: vec![crate::ast::Statement {
            source: Source::Current,
            address: None,
            receiver: crate::ast::Receiver::Rec,
            calls: Vec::new(),
        }],
    };
    crate::engine::eval_pipeline_with_configs(&pipeline, input, input, &rec_cfg, &grid_cfg)
}

fn assert_run(expr: &str, input: &str, expected: &str) {
    let out = run(expr, input).unwrap_or_else(|err| panic!("`{expr}` should succeed: {err}"));
    assert_eq!(out, expected);
}

fn assert_parse_err(expr: &str, expected_parts: &[&str]) {
    let err = parse_program(expr).expect_err(&format!("`{expr}` should fail to parse"));
    let msg = err.to_string();
    for part in expected_parts {
        assert!(
            msg.contains(part),
            "expected parse error for `{expr}` to contain `{part}`, got: {msg}"
        );
    }
}

fn run_with_fs(expr: &str, input: &str, fs: &str) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let mut rec_cfg = RecConfig::default();
    rec_cfg.fs = fs.to_string();
    let grid_cfg = GridConfig::default();
    let mut current = input.to_string();
    for pipeline in &stmts {
        current = crate::engine::eval_pipeline_with_configs(
            pipeline, &current, input, &rec_cfg, &grid_cfg,
        )?;
        outputs.push(current.clone());
    }
    Ok(join_outputs(&outputs))
}

fn run_with_configs(
    expr: &str,
    input: &str,
    rec_cfg: RecConfig,
    grid_cfg: GridConfig,
) -> Result<String> {
    let stmts = parse_program(expr)?;
    let mut outputs = Vec::new();
    let mut current = input.to_string();
    for pipeline in &stmts {
        current = crate::engine::eval_pipeline_with_configs(
            pipeline, &current, input, &rec_cfg, &grid_cfg,
        )?;
        outputs.push(current.clone());
    }
    Ok(join_outputs(&outputs))
}

fn join_outputs(outputs: &[String]) -> String {
    outputs.join("---\n")
}

// README-driven regression tests
// These protect representative README examples from silently drifting.

#[test]
fn readme_quick_start_classic_works() {
    let out = run_with_fs(
        r#"r.p(1,2,3).ofs("|")"#,
        "A,10;tokyo\nB:20;osaka\n",
        r#"[,;:]"#,
    )
    .expect("README Quick Start classic example should succeed");
    assert_eq!(out, "A|10|tokyo\nB|20|osaka\n");
}

#[test]
fn readme_quick_start_shorthand_works() {
    let out = run_with_fs(
        r#"r.p:1,2,3.ofs=|"#,
        "A,10;tokyo\nB:20;osaka\n",
        r#"[,;:]"#,
    )
    .expect("README Quick Start shorthand example should succeed");
    assert_eq!(out, "A|10|tokyo\nB|20|osaka\n");
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
fn readme_dsl_shape_stdin_address_example_works() {
    assert_run(r#"stdin.2r.n:1"#, "A 10\nB 20\nC 30\n", "1 B 20\n");
}

#[test]
fn readme_dsl_shape_regex_address_example_works() {
    assert_run(
        r#"/tokyo/r.p:1,2"#,
        "A 10 tokyo\nB 20 osaka\nC 30 tokyo\n",
        "A 10\nC 30\n",
    );
}

#[test]
fn readme_dsl_shape_grid_chain_example_works() {
    assert_run(
        r#"g.t().rt("r").m(p("K"),"orth","*")"#,
        "...\n.K.\n...\n",
        ".*.\n*K*\n.*.\n",
    );
}

#[test]
fn semicolon_runs_statements_sequentially_and_prints_all_outputs() {
    assert_run(
        r#"r.x(2,",").g(1,s(2)); r.n(1)"#,
        "A 10,20\nB 7,8\n",
        "A 30\nB 15\n---\n1 A 30\n2 B 15\n",
    );
}

#[test]
fn stdin_source_restarts_from_original_input() {
    assert_run(
        r#"r.x(2,",").g(1,s(2)); stdin.r.n(1)"#,
        "A 10,20\nB 7,8\n",
        "A 30\nB 15\n---\n1 A 10,20\n2 B 7,8\n",
    );
}

#[test]
fn prev_source_uses_previous_statement_output() {
    assert_run(
        r#"g.m(pt(3,3),"orth","X"); prev.1,5r.ch.ci("X").n(1)"#,
        "     \n     \n     \n     \n     \n",
        "  X  \n  X  \nXX XX\n  X  \n  X  \n---\n1 1\n2 1\n3 4\n4 1\n5 1\n",
    );
}

#[test]
fn readme_multiple_statements_with_stdin_reset_works() {
    assert_run(
        r#"r.x(2,",").g(1,s(2)); stdin.r.n(1)"#,
        "A 10,20\nB 7,8\n",
        "A 30\nB 15\n---\n1 A 10,20\n2 B 7,8\n",
    );
}

#[test]
fn readme_record_rs_example_works() {
    assert_run(r#"r.rs("|")"#, "A 10|B 20", "A 10\nB 20\n");
}

#[test]
fn readme_record_ors_example_works() {
    assert_run(r#"r.ors("|")"#, "A 10\nB 20\n", "A 10|B 20|");
}

#[test]
fn readme_record_select_example_works() {
    assert_run(
        r#"r.p(1,3)"#,
        "A 10 tokyo\nB 20 osaka\n",
        "A tokyo\nB osaka\n",
    );
}

#[test]
fn readme_record_replace_example_works() {
    assert_run(r#"r.sb("[0-9]","X")"#, "A-10\nB-20\n", "A-XX\nB-XX\n");
}

#[test]
fn readme_record_reshape_w2l_example_works() {
    assert_run(
        r#"r.sh("w2l",2)"#,
        "name math eng\nA 80 90\nB 70 85\n",
        "A math 80\nA eng 90\nB math 70\nB eng 85\n",
    );
}

#[test]
fn readme_record_flatten_example_works() {
    assert_run(
        r#"r.f("{name}:{age}")"#,
        "name age\nalice 20\nbob 30\ncarol 25\ndave 41\n",
        "alice:20\nbob:30\ncarol:25\ndave:41\n",
    );
}

#[test]
fn readme_grid_fs_example_works() {
    assert_run(r#"g.fs(",").ofs("|")"#, "a,b,c\nd,e,f\n", "a|b|c\nd|e|f\n");
}

#[test]
fn readme_grid_rs_example_works() {
    assert_run(r#"g.rs("|")"#, "abc|def|ghi", "abc\ndef\nghi\n\n");
}

#[test]
fn readme_grid_ors_example_works() {
    assert_run(r#"g.ors("---\n")"#, "abc\ndef\n", "abc---\ndef---\n");
}

#[test]
fn readme_grid_rotate_180_example_works() {
    assert_run(r#"g.rt("180")"#, "abc\ndef\nghi\n", "ihg\nfed\ncba\n");
}

// Parser diagnostics

#[test]
fn invalid_receiver_prefix_error_is_helpful() {
    assert_parse_err(
        r#"d.t"#,
        &[
            "invalid receiver prefix",
            "`d.t`",
            "expected `r.` / `rec.` or `g.` / `grid.`",
        ],
    );
}

#[test]
fn missing_closing_paren_error_points_to_segment() {
    assert_parse_err(
        r#"r.p(1,2"#,
        &["missing closing `)`", "`p(1,2`", "switch to shorthand"],
    );
}

#[test]
fn malformed_colon_shorthand_error_is_helpful() {
    assert_parse_err(
        r#"r.p:"#,
        &[
            "malformed shorthand call",
            "`p:`",
            "Use bare `name` for zero-arg calls",
        ],
    );
}

#[test]
fn malformed_equals_shorthand_error_is_helpful() {
    assert_parse_err(
        r#"r.ofs="#,
        &[
            "malformed shorthand call",
            "`ofs=`",
            "Use bare `name` for zero-arg calls",
        ],
    );
}

#[test]
fn empty_call_segment_error_is_helpful() {
    assert_parse_err(
        r#"r.p:1..ofs=|"#,
        &[
            "misused `.`",
            "empty call segment",
            "Remove the extra `.` or add a method name between dots",
        ],
    );
}

#[test]
fn empty_pipeline_stage_error_is_helpful() {
    assert_parse_err(
        r#"r.p:1 | "#,
        &[
            "misused `|`",
            "empty pipeline stage",
            "Add a receiver after `|`",
        ],
    );
}

#[test]
fn empty_statement_error_is_helpful() {
    assert_parse_err(
        r#"r.p:1;;g.t"#,
        &[
            "misused `;`",
            "empty statement",
            "Remove the extra `;` or add another statement after it",
        ],
    );
}

#[test]
fn invalid_bare_call_name_error_is_helpful() {
    assert_parse_err(
        r#"r.1bad"#,
        &[
            "invalid bare call name `1bad`",
            "`1bad`",
            "must start with a letter or `_`",
        ],
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
fn mark_can_use_point_function_origin() {
    assert_run(
        r#"g.m(pt(3,2),"orth","*")"#,
        ".....\n..K..\n.....\n",
        "..*..\n**K**\n..*..\n",
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
fn get_can_read_by_point_function() {
    assert_run(r#"g.get(pt(3,2))"#, "abc\ndef\nghi\n", "f\n");
}

#[test]
fn set_can_write_by_coordinates() {
    assert_run(r#"g.set(3,2,"X")"#, "abc\ndef\nghi\n", "abc\ndeX\nghi\n");
}

#[test]
fn set_can_write_by_point_function() {
    assert_run(r#"g.set(pt(3,2),"X")"#, "abc\ndef\nghi\n", "abc\ndeX\nghi\n");
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
fn transpose_can_pad_before_transposing() {
    assert_run(r#"g.t(pad("."))"#, "ab\ncde\n", "ac\nbd\n.e\n");
}

#[test]
fn rotate_can_pad_before_rotating() {
    assert_run(r#"g.rt("r",pad("."))"#, "ab\ncde\n", "ca\ndb\ne.\n");
}

#[test]
fn grid_can_pad_all_sides_evenly() {
    assert_run(r#"g.pad(1,".")"#, "ab\ncde\n", ".....\n.ab..\n.cde.\n.....\n");
}

#[test]
fn grid_can_pad_each_side_individually() {
    assert_run(r#"g.pd:1,0,2,1,".""#, "ab\ncde\n", "......\n..ab..\n..cde.\n");
}

#[test]
fn align_can_left_align_to_widest_row() {
    assert_run(r#"g.align("left",pad("."))"#, "a\nbbb\ncc\n", "a..\nbbb\ncc.\n");
}

#[test]
fn align_can_center_align_to_widest_row() {
    assert_run(r#"g.align("center",pad("."))"#, "a\nbbb\ncc\n", ".a.\nbbb\ncc.\n");
}

#[test]
fn align_can_right_align_to_widest_row() {
    assert_run(r#"g.al:r,pad:".""#, "a\nbbb\ncc\n", "..a\nbbb\n.cc\n");
}

#[test]
fn align_can_target_a_single_row() {
    assert_run(
        r#"g.align("center",rows(1),pad("."))"#,
        "a\nbbb\ncc\n",
        ".a.\nbbb\ncc.\n",
    );
}

#[test]
fn align_can_target_a_row_range_with_shorthand() {
    assert_run(
        r#"g.al:r,rows:"1:2",pad:".""#,
        "a\nbb\nccc\n",
        "..a\n.bb\nccc\n",
    );
}

#[test]
fn align_rows_out_of_range_is_rejected() {
    let err = run(r#"g.al:c,rows:4,pad:".""#, "a\nbbb\ncc\n")
        .expect_err("out-of-range align rows should fail");
    assert!(
        err.to_string().contains("rows out of range"),
        "unexpected error: {err}"
    );
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
fn line_can_write_from_point_function() {
    assert_run(
        r#"g.line(pt(2,2),"r","A","B","C")"#,
        ".....\n.....\n.....\n",
        ".....\n.ABC.\n.....\n",
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
        err.to_string().contains("invalid receiver prefix"),
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

#[test]
fn chars_splits_each_record_into_single_character_fields() {
    assert_run(r#"r.ch.p:1,3"#, "abc\nXYZ\n", "a c\nX Z\n");
}

#[test]
fn countif_counts_matching_fields_per_record() {
    assert_run(r#"r.ci("a")"#, "a b a\nc a d\n", "2\n1\n");
}

#[test]
fn chars_then_countif_counts_character_occurrences_per_line() {
    assert_run(r#"r.ch.ci("a")"#, "abca\nzzzz\n", "2\n0\n");
}

#[test]
fn grid_output_can_be_piped_into_record_chars_and_countif() {
    assert_run(
        r#"g.m(pt(3,3),"orth","X") | r.ch.ci("X")"#,
        "     \n     \n     \n     \n     \n",
        "1\n1\n4\n1\n1\n",
    );
}

#[test]
fn address_can_select_single_record_before_record_dsl() {
    assert_run(r#"2r.n:1"#, "A 10\nB 20\nC 30\n", "1 B 20\n");
}

#[test]
fn address_can_select_numeric_range_before_record_dsl() {
    assert_run(r#"1,2r.p:1"#, "A 10\nB 20\nC 30\n", "A\nB\n");
}

#[test]
fn address_can_select_to_last_record() {
    assert_run(r#"2,$r.p:1"#, "A 10\nB 20\nC 30\n", "B\nC\n");
}

#[test]
fn address_can_filter_records_by_regex() {
    assert_run(r#"/^B /r.p:1"#, "A 10\nB 20\nB 30\n", "B\nB\n");
}

#[test]
fn address_can_mix_numeric_and_regex_range() {
    assert_run(r#"2,/^C /r.p:1"#, "A 10\nB 20\nC 30\nD 40\n", "B\nC\n");
}

#[test]
fn address_respects_record_separator_option() {
    let mut rec_cfg = RecConfig::default();
    rec_cfg.rs = "|".to_string();
    let out = run_with_configs(r#"2r.p:1"#, "A 10|B 20|C 30|", rec_cfg, GridConfig::default())
        .expect("addressed record selection should respect custom rs");
    assert_eq!(out, "B\n");
}

#[test]
fn address_is_not_supported_for_grid_statements() {
    let err = run(r#"2g.t"#, "ab\ncd\n").expect_err("addressed grid statement should fail");
    assert!(
        err.to_string()
            .contains("addresses are currently only supported for record statements"),
        "unexpected error: {err}"
    );
}
