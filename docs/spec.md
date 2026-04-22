# rkg DSL Specification for v0.3.x

This document describes the DSL behavior implemented by `rkg` as of the v0.3.x line.

It is intentionally practical:

- examples come first
- parser behavior wins over idealized grammar
- ambiguous or surprising behavior is called out explicitly

## Scope

This specification covers the current handwritten parser in `src/parser.rs` and the related runtime checks in `src/engine.rs`.

It describes what is implemented now. It does not reserve or promise syntax that does not exist in the codebase yet.

## Program Shape

A program is a sequence of statements separated by `;`.

Each statement is a pipeline of one or more stages separated by `|`.

Each stage starts with a receiver:

- `r.` or `rec.` for record mode
- `g.` or `grid.` for grid mode

Examples:

```text
r.p:1,2,3
g.t.rt:r
r.p:1 | g.t
r.g:1,sum:2; prev.r.n:1
```

## Receiver Forms

Supported receiver forms:

```text
r.
rec.
g.
grid.
```

Optional source prefixes may appear before a receiver:

```text
stdin.r.p:1
prev.g.t
```

The parser also accepts a syntactic named source prefix:

```text
foo.r.p:1
```

But named sources are not executable yet. They currently fail at runtime with `named source is not supported yet`.

### Record Pre-Addressing

Record statements may start with an address before `r.` / `rec.`:

```text
2r.p:1
1,3r.p:1
2,$r.p:1
/tokyo/r.p:1,2
2,/^C /r.p:1
```

Supported address forms:

- `N` for a 1-based record number
- `$` for the last record
- `/regex/` for regex matching
- `A,B` for a range

Notes:

- a single regex address selects all matching records
- a range with a regex end selects from the start address to the first later match
- addresses use the current record separator (`rs`)
- `0` is invalid

Grid statements do not support addresses at execution time:

```text
2g.t
```

This parses as a statement prefix, but execution fails with `addresses are currently only supported for record statements`.

## Call Forms

Each stage contains method calls separated by `.`.

### Classic Call Syntax

Classic calls use parentheses:

```text
r.p(1,2,3)
g.rt("r")
r.fs(",").ofs("|")
```

### Shorthand Syntax

Colon form:

```text
method:arg1,arg2
```

Equivalent to:

```text
method(arg1,arg2)
```

Examples:

```text
r.p:1,2,3
r.g:1,sum:2
g.rt:r
g.m:p("K"),"diag","*"
```

Equals form:

```text
method=value
```

Equivalent to a single-argument call:

```text
method(value)
```

Examples:

```text
r.ofs=|
r.fs=,
g.rv:h
```

### Bare Call Syntax

A bare method name means a zero-argument call:

```text
g.t
r.ch
```

Equivalent to:

```text
g.t()
r.ch()
```

## How Punctuation Is Interpreted

### `.`

`.` separates method calls inside a single stage.

```text
r.p:1,2.ofs=|
g.t.rt:r
```

`.` is only a separator at top level. It is ignored inside:

- double-quoted strings
- parentheses

### `|`

`|` separates pipeline stages.

```text
r.p:1,2.ofs=- | g.t
```

Important detail: `|` is only treated as a pipeline separator when the text on its right can start a valid statement prefix. In practice, that means the right side must begin with something like:

- `r.` / `rec.`
- `g.` / `grid.`
- `stdin.` followed by a valid receiver
- `prev.` followed by a valid receiver
- an address followed by a valid receiver

If not, the `|` stays inside the current argument text.

### `;`

`;` separates statements.

```text
r.g:1,sum:2; prev.r.n:1
```

Execution is sequential:

1. each statement runs after the previous statement
2. `prev.` refers to the previous statement output
3. `stdin.` restarts from the original stdin input

When multiple statements are printed, `rkg` prints `---` between statement outputs.

### `:`

At top level within a call, `:` starts shorthand call syntax.

```text
p:1,2,3
rt:r
```

The parser uses the first top-level `:` or `=` as the shorthand delimiter. If both appear, the earlier one wins.

Inside arguments, `:` is ordinary text unless it appears at top level for a nested shorthand call.

Examples:

```text
r.g:1,sum:2
g.m:p("K"),"line","r"
```

### `=`

At top level within a call, `=` also starts shorthand call syntax.

```text
ofs=|
fs=,
```

Like `:`, it is only recognized at top level, not inside double quotes or nested parentheses.

## Quoting and Escaping

### Strings

Double quotes create string literals:

```text
r.fs(",")
g.rt("r")
g.m(p("K"),"diag","*")
g.align("right",rows(2,4),pad("."))
```

Current parser behavior:

- only double quotes are recognized as string delimiters
- single quotes are not DSL string delimiters
- `\"` is unescaped inside a quoted string
- other backslashes are kept as-is by the parser

Example:

```text
r.ofs("\"")
```

produces a string argument containing `"`.

### Escape Sequences Used Later by Runtime

Some methods and separators later pass string values through `unescape`, which currently converts:

- `\t` to tab
- `\n` to newline
- `\r` to carriage return

This is runtime behavior, not general parser escaping.

In other words:

- the parser itself only decodes `\"`
- `\t`, `\n`, and `\r` become special only where the called operation treats the argument as a string separator/value and runs unescaping

### Parentheses

Parentheses group expressions and protect inner punctuation from top-level splitting:

```text
r.g(1,sum(2))
r.g:1,sum:2
g.m:p("K",2),"diag","*"
```

The parser also strips one outer pair of parentheses around an expression:

```text
(1)
("x")
(sum:2)
```

### Shell Quoting

The DSL parser does not know about shell parsing. In practice, pass the whole DSL as one shell argument when it contains characters your shell treats specially.

Examples:

```bash
rkg 'r.p:1,2.ofs=|'
rkg 'g.m:p("K"),"diag","*"'
```

## Expression Kinds

Arguments are parsed as one of:

- string literal, such as `"x"`
- integer, such as `1` or `-2`
- nested call, such as `sum:2` or `p("K")`
- identifier/text, such as `row`, `orth`, `*`, `|`

A bare token does not need quotes unless you need shell protection or you need the DSL to preserve punctuation that would otherwise split the expression.

## Valid Syntax Examples

```text
r.p:1,2,3
r.p(1,2,3).ofs("|")
r.p:1,2,3.ofs=|
g.t.rt:r
r.g:1,sum:2
g.m:p("K"),"diag","*"
g.al:r,rows:"2:4",pad:"."
stdin.r.n:1
prev.1,5r.ch.ci("X").n(1)
2r.n:1
/tokyo/r.p:1,2
r.p:1,2.ofs=- | g.t
r.g:1,sum:2; prev.r.n:1
```

## Invalid or Rejected Examples

These reflect current behavior, including runtime rejections.

```text
d.t
```

Invalid receiver. Statements must start with `r.` / `rec.` or `g.` / `grid.`.

```text
0r.p:1
```

Invalid address. Record positions are 1-based.

```text
/tokyo r.p:1
```

Invalid address form. Regex addresses must be written as `/.../` directly before the receiver.

```text
2g.t
```

Parses, but fails at runtime because addressed grid statements are not supported.

```text
stdin.foo
```

Invalid statement prefix. A source prefix must still be followed by a valid receiver form.

```text
/abc
```

Invalid statement. An address alone is not a statement.

## Ambiguities and Current Behavior

### Named Sources

The parser accepts source names like `name.r...`, but execution does not support them yet.

### Addressed Grid Statements

The parser accepts `2g.t`-style prefixes, but the engine rejects them.

### Top-Level Splitting Rules

The parser splits on `.`, `|`, `;`, `,`, `:`, and `=` only at top level. "Top level" means:

- not inside double quotes
- not inside parentheses

This is why nested shorthand calls such as `sum:2` work inside outer shorthand calls.

### Unterminated or Mismatched Input

The parser has explicit errors for some malformed input, such as missing `)` in a parenthesized call or an unterminated regex address. For other malformed combinations, behavior follows the current handwritten parser rather than a separate formal grammar.

When in doubt, prefer examples from this document and the current test suite.

## Compatibility Policy for v0.3.x

For the v0.3.x series:

- the documented syntax in this file is the compatibility target
- existing documented programs should keep their meaning across patch releases
- undocumented behavior, parser edge cases, and not-yet-supported constructs may change in patch releases
- if code and documentation disagree, the implementation in the current release is the source of truth until the docs are updated

That policy is intentionally conservative while the DSL is still growing.
