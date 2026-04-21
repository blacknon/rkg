# rkg Cheat Sheet

Small, copy-pastable examples for the most common `rkg` workflows.

See also:

- [README](../README.md)
- [DSL spec](spec.md)
- [GitHub Releases](https://github.com/blacknon/rkg/releases)

## Mental Model

- `r.` / `rec.`: record mode for rows and fields
- `g.` / `grid.`: grid mode for line and cell geometry
- `.` chains methods
- `|` pipes one stage into the next
- `;` starts the next statement
- `stdin.` restarts from original stdin
- `prev.` explicitly uses the previous statement output

## Install

```bash
cargo install rkg
```

Build from source:

```bash
cargo build --release
```

## Quick Start

Classic:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  rkg -F '[,:;]' 'r.p(1,2,3).ofs("|")'
```

Shorthand:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  rkg -F '[,:;]' 'r.p:1,2,3.ofs=|'
```

## Record Mode

Select columns:

```bash
printf 'A 10 tokyo\nB 20 osaka\n' |
  rkg 'r.p:1,3'
```

Explode one field into many rows:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg -F, -O, 'r.x:2,";"'
```

Group and sum:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g:1,s:2'
```

Reshape wide to long:

```bash
printf 'name math eng\nA 80 90\nB 70 85\n' |
  rkg 'r.sh:w2l,2'
```

Flatten with headers:

```bash
printf 'name age\nalice 20\nbob 30\n' |
  rkg 'r.f:"{name}:{age}"'
```

## Grid Mode

Transpose:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.t'
```

Rotate right:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.rt:r'
```

Mark orthogonal rays from a point:

```bash
printf '.......\n.......\n...K...\n.......\n.......\n' |
  rkg 'g.m:p("K"),"orth","*"'
```

Write a line from coordinates:

```bash
printf '.....\n.....\n.....\n' |
  rkg 'g.ln:2,2,r,A,B,C'
```

Pad a ragged grid:

```bash
printf 'ab\ncde\n' |
  rkg 'g.pd:1,"."'
```

## Common Shorthand

```text
r.p:1,3        # select fields 1 and 3
r.ofs=|        # set output field separator
g.t            # zero-arg call
g.rt:r         # rotate right
g.rv:h         # reverse horizontally
g.ln:2,2,r,A,B # line(...)
```

Classic equivalents:

```text
r.p(1,3)
r.ofs("|")
g.t()
g.rt("r")
g.rev("h")
g.line(2,2,"r","A","B")
```

## Sources and Statements

Use a later statement on the previous result:

```bash
printf 'A 10,20\nB 7,8\n' |
  rkg 'r.x(2,",").g(1,s(2)); r.n(1)'
```

Restart from original stdin:

```bash
printf 'A 10,20\nB 7,8\n' |
  rkg 'r.x(2,",").g(1,s(2)); stdin.r.n(1)'
```

Pre-address records before `r.`:

```bash
printf 'A 10\nB 20\nC 30\n' |
  rkg '2r.n:1'
```

Regex address:

```bash
printf 'A 10 tokyo\nB 20 osaka\nC 30 tokyo\n' |
  rkg '/tokyo/r.p:1,2'
```
