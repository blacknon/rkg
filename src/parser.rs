use anyhow::{anyhow, bail, Result};

use crate::ast::{Call, Expr, Receiver, Statement};

enum CallSyntax<'a> {
    Paren { name: String, inner: &'a str },
    Shorthand { name: String, inner: &'a str },
    Bare { name: String },
}

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
    match parse_call_syntax(src)? {
        CallSyntax::Bare { name } => Ok(Call {
            name,
            args: Vec::new(),
        }),
        CallSyntax::Paren { name, inner } | CallSyntax::Shorthand { name, inner } => {
            let args = parse_args(inner)?;
            Ok(Call { name, args })
        }
    }
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
    if is_call_syntax(s) {
        return Ok(Expr::Call(parse_call(s)?));
    }
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Expr::Num(n));
    }
    Ok(Expr::Ident(s.to_string()))
}

fn is_call_syntax(src: &str) -> bool {
    src.contains('(') || find_shorthand_delim(src).is_some()
}

fn parse_call_syntax(src: &str) -> Result<CallSyntax<'_>> {
    if let Some(open) = src.find('(') {
        let name = src[..open].trim().to_string();
        ensure_call_name(&name, src)?;
        let close = src
            .rfind(')')
            .ok_or_else(|| anyhow!("expected ')' in call: {src}"))?;
        return Ok(CallSyntax::Paren {
            name,
            inner: &src[open + 1..close],
        });
    }

    if let Some((idx, _delim)) = find_shorthand_delim(src) {
        let name = src[..idx].trim().to_string();
        ensure_call_name(&name, src)?;
        return Ok(CallSyntax::Shorthand {
            name,
            inner: &src[idx + 1..],
        });
    }

    let name = src.trim().to_string();
    ensure_call_name(&name, src)?;
    Ok(CallSyntax::Bare { name })
}

fn parse_args(src: &str) -> Result<Vec<Expr>> {
    if src.trim().is_empty() {
        return Ok(Vec::new());
    }

    split_top_level(src, ',')
        .into_iter()
        .map(|arg| parse_expr(arg.trim()))
        .collect()
}

fn find_shorthand_delim(src: &str) -> Option<(usize, char)> {
    let colon = find_top_level(src, ':').map(|idx| (idx, ':'));
    let equals = find_top_level(src, '=').map(|idx| (idx, '='));
    match (colon, equals) {
        (Some(c), Some(e)) => Some(if c.0 < e.0 { c } else { e }),
        (Some(c), None) => Some(c),
        (None, Some(e)) => Some(e),
        (None, None) => None,
    }
}

fn ensure_call_name(name: &str, src: &str) -> Result<()> {
    if name.is_empty() {
        bail!("empty call: {src}");
    }
    Ok(())
}

fn find_top_level(src: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = '\0';

    for (idx, ch) in src.char_indices() {
        match ch {
            '"' if prev != '\\' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => depth -= 1,
            _ if ch == target && !in_str && depth == 0 => return Some(idx),
            _ => {}
        }
        prev = ch;
    }

    None
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
