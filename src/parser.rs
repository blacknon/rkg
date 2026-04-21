use anyhow::{anyhow, bail, Result};

use crate::ast::{Address, AddressRange, Call, Expr, Pipeline, Receiver, Source, Statement};

enum CallSyntax<'a> {
    Paren { name: String, inner: &'a str },
    Shorthand { name: String, inner: &'a str },
    Bare { name: String },
}

pub fn parse_program(src: &str) -> Result<Vec<Pipeline>> {
    // `;` is the outermost separator, so we validate and split in program order.
    validate_nonempty_top_level_segments(
        src,
        ';',
        "statement",
        "Remove the extra `;` or add another statement after it.",
    )?;
    let stmts = split_top_level(src, ';');
    let mut out = Vec::new();
    for stmt in stmts {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        out.push(parse_pipeline(s)?);
    }
    Ok(out)
}

fn parse_pipeline(src: &str) -> Result<Pipeline> {
    // `|` is only a real pipeline boundary when the right-hand side can start a
    // new statement. This keeps `ofs=|` and similar shorthand forms working.
    validate_pipeline_boundaries(src)?;
    let stages = split_pipeline_stages(src)
        .into_iter()
        .map(|stage| parse_statement(stage.trim()))
        .collect::<Result<Vec<_>>>()?;
    Ok(Pipeline { stages })
}

fn parse_statement(src: &str) -> Result<Statement> {
    let (source, address, receiver, rest) = parse_statement_prefix(src)?;

    // Once the prefix is consumed, the remainder is a dot-chained call list.
    if !rest.trim().is_empty() {
        validate_nonempty_top_level_segments(
            rest,
            '.',
            "call segment",
            "Remove the extra `.` or add a method name between dots.",
        )?;
    }

    let parts = split_top_level(rest, '.');
    let calls = parts
        .into_iter()
        .map(|part| parse_call(part.trim()))
        .collect::<Result<Vec<_>>>()?;

    Ok(Statement {
        source,
        address,
        receiver,
        calls,
    })
}

fn split_pipeline_stages(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut cur = String::new();
    let mut prev = '\0';

    for (idx, ch) in src.char_indices() {
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
            '|' if !in_str && depth == 0 && starts_with_receiver(&src[idx + ch.len_utf8()..]) => {
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

fn starts_with_receiver(src: &str) -> bool {
    let s = src.trim_start();
    parse_statement_prefix(s).is_ok()
}

fn parse_statement_prefix(src: &str) -> Result<(Source, Option<AddressRange>, Receiver, &str)> {
    let (source, rest) = parse_source_prefix(src);

    if let Some(rest) = rest.strip_prefix("r.") {
        return Ok((source, None, Receiver::Rec, rest));
    }
    if let Some(rest) = rest.strip_prefix("rec.") {
        return Ok((source, None, Receiver::Rec, rest));
    }
    if let Some(rest) = rest.strip_prefix("g.") {
        return Ok((source, None, Receiver::Grid, rest));
    }
    if let Some(rest) = rest.strip_prefix("grid.") {
        return Ok((source, None, Receiver::Grid, rest));
    }

    let first = rest.chars().next().unwrap_or_default();
    if !matches!(first, '$' | '/' | '0'..='9') {
        bail!("{}", invalid_receiver_prefix_message(src));
    }

    let (address, rest) = parse_address_range(rest)?;
    if let Some(rest) = rest.strip_prefix("r.") {
        return Ok((source, Some(address), Receiver::Rec, rest));
    }
    if let Some(rest) = rest.strip_prefix("rec.") {
        return Ok((source, Some(address), Receiver::Rec, rest));
    }
    if let Some(rest) = rest.strip_prefix("g.") {
        return Ok((source, Some(address), Receiver::Grid, rest));
    }
    if let Some(rest) = rest.strip_prefix("grid.") {
        return Ok((source, Some(address), Receiver::Grid, rest));
    }

    bail!("{}", invalid_receiver_prefix_message(src))
}

fn parse_source_prefix(src: &str) -> (Source, &str) {
    // `stdin.` and `prev.` are the only executable source prefixes today.
    // Named sources are parsed for forward compatibility, but the engine still
    // rejects them at runtime.
    if let Some(rest) = src.strip_prefix("stdin.") {
        return (Source::Stdin, rest);
    }
    if let Some(rest) = src.strip_prefix("prev.") {
        return (Source::Prev, rest);
    }

    if let Some(dot_idx) = find_top_level(src, '.') {
        let candidate = &src[..dot_idx];
        if is_source_name(candidate) && !matches!(candidate, "r" | "rec" | "g" | "grid") {
            return (Source::Named(candidate.to_string()), &src[dot_idx + 1..]);
        }
    }

    (Source::Current, src)
}

fn is_source_name(src: &str) -> bool {
    let mut chars = src.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn parse_address_range(src: &str) -> Result<(AddressRange, &str)> {
    let (start, rest) = parse_address(src)?;
    if let Some(rest) = rest.strip_prefix(',') {
        let (end, rest) = parse_address(rest)?;
        Ok((
            AddressRange {
                start,
                end: Some(end),
            },
            rest,
        ))
    } else {
        Ok((AddressRange { start, end: None }, rest))
    }
}

fn parse_address(src: &str) -> Result<(Address, &str)> {
    if src.is_empty() {
        bail!("missing address");
    }

    let first = src.chars().next().unwrap_or_default();
    match first {
        '$' => Ok((Address::Last, &src['$'.len_utf8()..])),
        '/' => parse_regex_address(src),
        c if c.is_ascii_digit() => parse_numeric_address(src),
        _ => bail!("invalid address: {src}"),
    }
}

fn parse_numeric_address(src: &str) -> Result<(Address, &str)> {
    let end = src
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_ascii_digit()).then_some(idx))
        .unwrap_or(src.len());
    let value = src[..end].parse::<usize>()?;
    if value == 0 {
        bail!("address positions are 1-based: {src}");
    }
    Ok((Address::Line(value), &src[end..]))
}

fn parse_regex_address(src: &str) -> Result<(Address, &str)> {
    let mut escaped = false;
    for (idx, ch) in src.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '/' => {
                let pat = src[1..idx].replace(r"\/", "/");
                return Ok((Address::Regex(pat), &src[idx + 1..]));
            }
            _ => {}
        }
    }
    bail!("unterminated regex address: {src}")
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
    let shorthand = find_shorthand_delim(src);
    let open = src.find('(');

    // Shorthand takes precedence when its delimiter appears before a `(`.
    if let Some((idx, delim)) = shorthand {
        if open.map(|open_idx| idx < open_idx).unwrap_or(true) {
            let name = src[..idx].trim().to_string();
            ensure_shorthand_call_name(&name, src, delim)?;
            let inner = &src[idx + 1..];
            if inner.trim().is_empty() {
                bail!("{}", malformed_shorthand_message(src, delim));
            }
            return Ok(CallSyntax::Shorthand {
                name,
                inner,
            });
        }
    }

    if let Some(open_idx) = open {
        let name = src[..open_idx].trim().to_string();
        ensure_paren_call_name(&name, src)?;
        let close = src
            .rfind(')')
            .ok_or_else(|| anyhow!("{}", missing_closing_paren_message(src)))?;
        return Ok(CallSyntax::Paren {
            name,
            inner: &src[open_idx + 1..close],
        });
    }

    let name = src.trim().to_string();
    ensure_valid_call_name(&name, src)?;
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

fn ensure_shorthand_call_name(name: &str, src: &str, delim: char) -> Result<()> {
    if name.is_empty() {
        bail!("{}", malformed_shorthand_message(src, delim));
    }
    ensure_valid_call_name(name, src)
}

fn ensure_paren_call_name(name: &str, src: &str) -> Result<()> {
    if name.is_empty() {
        bail!(
            "invalid call segment `{src}`: missing method name before `(`. Add a method name, such as `p(...)`."
        );
    }
    ensure_valid_call_name(name, src)
}

fn ensure_valid_call_name(name: &str, src: &str) -> Result<()> {
    if !is_valid_call_name(name) {
        bail!("{}", invalid_call_name_message(name, src));
    }
    Ok(())
}

fn is_valid_call_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn validate_nonempty_top_level_segments(
    src: &str,
    delim: char,
    segment_kind: &str,
    suggestion: &str,
) -> Result<()> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = '\0';
    let mut saw_delim = false;
    let mut cur_has_content = false;

    for ch in src.chars() {
        match ch {
            '"' if prev != '\\' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => depth -= 1,
            _ if ch == delim && !in_str && depth == 0 => {
                saw_delim = true;
                if !cur_has_content {
                    bail!(
                        "misused `{delim}` in `{src}`: found an empty {segment_kind}. {suggestion}"
                    );
                }
                cur_has_content = false;
            }
            _ if !ch.is_whitespace() => cur_has_content = true,
            _ => {}
        }
        prev = ch;
    }

    if saw_delim && !cur_has_content {
        bail!("misused `{delim}` in `{src}`: found an empty {segment_kind}. {suggestion}");
    }

    Ok(())
}

fn validate_pipeline_boundaries(src: &str) -> Result<()> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = '\0';

    for (idx, ch) in src.char_indices() {
        match ch {
            '"' if prev != '\\' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => depth -= 1,
            '|' if !in_str && depth == 0 => {
                let left = &src[..idx];
                let right = &src[idx + ch.len_utf8()..];
                let spaced = left
                    .chars()
                    .last()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                    || right
                        .chars()
                        .next()
                        .map(|c| c.is_whitespace())
                        .unwrap_or(false);

                if spaced && right.trim().is_empty() {
                    bail!(
                        "misused `|` in `{src}`: found an empty pipeline stage. Add a receiver after `|`, such as `r.` or `g.`, or remove the extra `|`."
                    );
                }
            }
            _ => {}
        }
        prev = ch;
    }

    Ok(())
}

fn invalid_receiver_prefix_message(src: &str) -> String {
    format!(
        "invalid receiver prefix in `{src}`: expected `r.` / `rec.` or `g.` / `grid.`. Start the statement with a receiver, such as `r.method...` or `g.method...`."
    )
}

fn missing_closing_paren_message(src: &str) -> String {
    format!(
        "missing closing `)` in call segment `{src}`. Add the closing `)` or switch to shorthand like `name:arg1,arg2` if that reads better."
    )
}

fn malformed_shorthand_message(src: &str, delim: char) -> String {
    format!(
        "malformed shorthand call `{src}`: `{delim}` must follow a method name and be followed by at least one argument. Use bare `name` for zero-arg calls, or write `name{delim}value` / `name(...)`."
    )
}

fn invalid_call_name_message(name: &str, src: &str) -> String {
    format!(
        "invalid bare call name `{name}` in `{src}`: call names must start with a letter or `_`, then use only letters, digits, or `_`. Rename the method token or check for misplaced punctuation."
    )
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
