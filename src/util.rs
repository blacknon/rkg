use anyhow::{bail, Context, Result};

use crate::ast::{Call, Expr};

pub fn arg_string(call: &Call, idx: usize) -> Result<String> {
    call.args
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("missing arg {} for {}", idx + 1, call.name))
        .and_then(expr_to_string)
}

pub fn expr_to_string(expr: &Expr) -> Result<String> {
    match expr {
        Expr::Str(s) => Ok(s.clone()),
        Expr::Num(n) => Ok(n.to_string()),
        Expr::Ident(s) => Ok(s.clone()),
        Expr::Call(_) => bail!("nested call cannot be converted to string here"),
    }
}

pub fn arg_usize1(call: &Call, idx: usize) -> Result<usize> {
    arg_usize_expr(call, idx)
}

pub fn arg_usize_expr(call: &Call, idx: usize) -> Result<usize> {
    match call.args.get(idx) {
        Some(Expr::Num(n)) if *n > 0 => Ok(*n as usize),
        Some(Expr::Str(s)) => s
            .parse::<usize>()
            .with_context(|| format!("expected integer arg for {}", call.name)),
        _ => bail!(
            "expected positive integer arg {} for {}",
            idx + 1,
            call.name
        ),
    }
}

pub fn parse_num(s: &str) -> Result<f64> {
    s.parse::<f64>()
        .with_context(|| format!("not a number: {s}"))
}

pub fn trim_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

pub fn unescape(s: &str) -> String {
    s.replace(r"\t", "\t")
        .replace(r"\n", "\n")
        .replace(r"\r", "\r")
}

pub fn split_keep_nonempty<'a>(input: &'a str, sep: &str) -> Vec<&'a str> {
    if sep == "\n" {
        input.lines().collect()
    } else {
        input.split(sep).filter(|s| !s.is_empty()).collect()
    }
}
