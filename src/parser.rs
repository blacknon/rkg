use anyhow::{anyhow, bail, Result};

use crate::ast::{Call, Expr, Receiver, Statement};

pub fn parse_program(src: &str) -> Result<Vec<Statement>> {
    let stmts = split_top_level(src, ';');
    let mut out = Vec::new();
    for stmt in stmts {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        out.push(parse_statement(s)?);
    }
    Ok(out)
}

fn parse_statement(src: &str) -> Result<Statement> {
    let (receiver, rest) = if let Some(rest) = src.strip_prefix("r.") {
        (Receiver::Rec, rest)
    } else if let Some(rest) = src.strip_prefix("rec.") {
        (Receiver::Rec, rest)
    } else if let Some(rest) = src.strip_prefix("d.") {
        (Receiver::Grid, rest)
    } else if let Some(rest) = src.strip_prefix("grid.") {
        (Receiver::Grid, rest)
    } else {
        bail!("statement must start with r./rec. or d./grid.: {src}")
    };

    let parts = split_top_level(rest, '.');
    let calls = parts
        .into_iter()
        .map(|part| parse_call(part.trim()))
        .collect::<Result<Vec<_>>>()?;

    Ok(Statement { receiver, calls })
}

fn parse_call(src: &str) -> Result<Call> {
    let open = src
        .find('(')
        .ok_or_else(|| anyhow!("expected '(' in call: {src}"))?;
    let close = src
        .rfind(')')
        .ok_or_else(|| anyhow!("expected ')' in call: {src}"))?;
    let name = src[..open].trim().to_string();
    let inner = &src[open + 1..close];
    let args = if inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level(inner, ',')
            .into_iter()
            .map(|arg| parse_expr(arg.trim()))
            .collect::<Result<Vec<_>>>()?
    };

    Ok(Call { name, args })
}

fn parse_expr(src: &str) -> Result<Expr> {
    let s = src.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return Ok(Expr::Str(
            s[1..s.len() - 1].replace(r#"\""#, '"'.to_string().as_str()),
        ));
    }
    if s.starts_with('(') && s.ends_with(')') {
        return parse_expr(&s[1..s.len() - 1]);
    }
    if s.contains('(') && s.ends_with(')') {
        return Ok(Expr::Call(parse_call(s)?));
    }
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Expr::Num(n));
    }
    Ok(Expr::Ident(s.to_string()))
}

fn split_top_level(src: &str, delim: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut cur = String::new();
    let mut prev = '\0';

    for ch in src.chars() {
        match ch {
            '"' if prev != '\\' => {
                in_str = !in_str;
                cur.push(ch);
            }
            '(' if !in_str => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_str => {
                depth -= 1;
                cur.push(ch);
            }
            _ if ch == delim && !in_str && depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
        prev = ch;
    }

    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }

    out
}
