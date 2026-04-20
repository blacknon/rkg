rkg
===

**`r`ecord** + **`k`nit** + **`g`rid**

## About


`rkg` is the short crate/bin name for a one-liner oriented record/grid processor.

This version implements the current function-level DSL with:

- `r.` / `rec.` for record mode
- `d.` / `grid.` for grid mode
- method chaining with `.`
- statement reset with `;`
- AWK-like separators: `fs`, `rs`, `ofs`, `ors`
- `-F` / `--field-separator` for AWK-like initial field separator override

## Build

```bash
cargo build --release
```

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  cargo run -- -F '[,:;]' 'r.p(1,2,3).ofs("|")'
```

## DSL shape

```text
r.fs(",").x(2,";").g(1,s(2)).ofs(",");
d.t().rt("r").m("K","rook","*")
```

- `;` resets evaluation to the original stdin for the next statement
- only the last statement is printed by default
- `--print-all` prints all statement results separated by `---`
- `-F` sets the initial record-mode `fs` before the DSL runs, and accepts regex patterns

## Record functions

- `fs(re)` input field separator regex, AWK-like (`\s+` by default)
- `rs(sep)` input record separator (`\n` by default)
- `ofs(sep)` output field separator (` ` by default)
- `ors(sep)` output record separator (`\n` by default)
- `p(...)` / `select(...)` select fields, e.g. `p(1,"3:")`
- `sb(re, rep)` / `replace(re, rep)` regex replace per cell
- `n(start_or_AZ)` / `enum(...)` prepend numbering or `A-Z` cycle labels
- `x(col, sep)` / `explode(...)` split one field into multiple rows
- `i(key_col, val_col, join_sep?)` / `implode(...)` collapse rows by key
- `g(key_col, agg...)` / `groupby(...)` aggregate by key
- `sh(mode, ...)` / `reshape(...)` where mode is `w2l` or `l2w`
- `f(template?)` / `flatten(...)` flatten records; optional template like `"{name}:{age}"`

## Aggregators

- `s(col)` / `sum(col)`
- `c()` / `count()`
- `mn(col)` / `min(col)`
- `mx(col)` / `max(col)`
- `a(col)` / `avg(col)`

## Grid functions

- `fs(sep)` optional cell separator; default is character grid
- `rs(sep)` / `ofs(sep)` / `ors(sep)`
- `t()` / `transpose()`
- `rt("r"|"l"|"180")` / `rotate(...)`
- `m(from, ray, put)` marks along a ray (`rook`, `bishop`, `queen`, `8`)
- `m(from, through_re, to, put)` 8-direction pattern mark, useful for reversi-like scans

## Examples

### Record functions

#### `-F re` / `--field-separator re`

Sets the initial record-mode field separator from the CLI before any DSL method runs.

Input:

```text
A,10;tokyo
B:20;osaka
```

Command:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  rkg -F '[,:;]' 'r.p(1,2,3).ofs("|")'
```

Output:

```text
A|10|tokyo
B|20|osaka
```

#### `fs(re)`

Splits each record with the given regex instead of the default whitespace separator.

Input:

```text
A,10
B,20
```

Command:

```bash
printf 'A,10\nB,20\n' |
  rkg 'r.fs(",")'
```

Output:

```text
A 10
B 20
```

#### `rs(sep)`

Treats the given separator as the boundary between input records.

Input:

```text
A 10|B 20|
```

Command:

```bash
printf 'A 10|B 20|' |
  rkg 'r.rs("|")'
```

Output:

```text
A 10
B 20
```

#### `ofs(sep)`

Changes the separator used when fields are joined for output.

Input:

```text
A 10
B 20
```

Command:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.ofs(",")'
```

Output:

```text
A,10
B,20
```

#### `ors(sep)`

Changes the separator used when output records are joined together.

Input:

```text
A 10
B 20
```

Command:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.ors("|")'
```

Output:

```text
A 10|B 20|
```

#### `p(...)` / `select(...)`

Keeps only the requested fields and removes the rest.

Input:

```text
A 10 tokyo
B 20 osaka
```

Command:

```bash
printf 'A 10 tokyo\nB 20 osaka\n' |
  rkg 'r.p(1,"3:")'
```

Output:

```text
A tokyo
B osaka
```

#### `sb(re, rep)` / `replace(re, rep)`

Replaces text matching the regex in every cell.

Input:

```text
A-10
B-20
```

Command:

```bash
printf 'A-10\nB-20\n' |
  rkg 'r.fs("-").sb("[0-9]","X").ofs("-")'
```

Output:

```text
A-XX
B-XX
```

#### `n(start_or_AZ)` / `enum(...)`

Adds a numeric counter column to the front of each row.

Input:

```text
A 10
B 20
```

Command:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.n(1)'
```

Output:

```text
1 A 10
2 B 20
```

Uses alphabet labels instead of numbers and prepends them as a new first column.

Input:

```text
A 10
B 20
C 30
```

Command:

```bash
printf 'A 10\nB 20\nC 30\n' |
  rkg 'r.n("A-Z")'
```

Output:

```text
A A 10
B B 20
C C 30
```

#### `x(col, sep)` / `explode(...)`

Splits one field into multiple rows while keeping the other columns as-is.

Input:

```text
A,10;20;30
B,7;8
```

Command:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg 'r.fs(",").x(2,";").ofs(",")'
```

Output:

```text
A,10
A,20
A,30
B,7
B,8
```

#### `i(key_col, val_col, join_sep?)` / `implode(...)`

Merges rows with the same key by joining one value column into a single field.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.i(1,2,",")'
```

Output:

```text
A 10,20
B 7
```

#### `g(key_col, agg...)` / `groupby(...)`

Groups rows by the key column and emits aggregate results per group.

Input:

```text
A,10;20;30
B,7;8
```

Command:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg 'r.fs(",").x(2,";").g(1,s(2)).ofs(",")'
```

Output:

```text
A,60
B,15
```

#### `sh("w2l", ...)` / `reshape(...)`

Turns wide columns into repeated long-form rows.

Input:

```text
name math eng
A 80 90
B 70 85
```

Command:

```bash
printf 'name math eng\nA 80 90\nB 70 85\n' |
  rkg 'r.sh("w2l",2)'
```

Output:

```text
A math 80
A eng 90
B math 70
B eng 85
```

#### `sh("l2w", ...)` / `reshape(...)`

Turns repeated long-form rows back into a wide table.

Input:

```text
A math 80
A eng 90
B math 70
B eng 85
```

Command:

```bash
printf 'A math 80\nA eng 90\nB math 70\nB eng 85\n' |
  rkg 'r.sh("l2w",2,3)'
```

Output:

```text
key eng math
A 90 80
B 85 70
```

#### `f(template?)` / `flatten(...)`

Renders each data row as a single string using the header names in the template.

Input:

```text
name age
alice 20
bob 30
```

Command:

```bash
printf 'name age\nalice 20\nbob 30\n' |
  rkg 'r.f("{name}:{age}")'
```

Output:

```text
alice:20
bob:30
```

### Aggregators

#### `s(col)` / `sum(col)`

Sums the numeric values in the target column for each group.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g(1,s(2))'
```

Output:

```text
A 30
B 7
```

#### `c()` / `count()`

Counts how many rows belong to each group.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g(1,c())'
```

Output:

```text
A 2
B 1
```

#### `mn(col)` / `min(col)`

Takes the smallest numeric value in the target column for each group.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g(1,mn(2))'
```

Output:

```text
A 10
B 7
```

#### `mx(col)` / `max(col)`

Takes the largest numeric value in the target column for each group.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g(1,mx(2))'
```

Output:

```text
A 20
B 7
```

#### `a(col)` / `avg(col)`

Computes the average numeric value in the target column for each group.

Input:

```text
A 10
A 20
B 7
```

Command:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g(1,a(2))'
```

Output:

```text
A 15
B 7
```

### Grid functions

#### `fs(sep)`

Treats each input row as separator-delimited cells instead of a character grid.

Input:

```text
a,b,c
d,e,f
```

Command:

```bash
printf 'a,b,c\nd,e,f\n' |
  rkg 'd.fs(",").ofs("|")'
```

Output:

```text
a|b|c
d|e|f
```

#### `rs(sep)`

Treats the given separator as the boundary between grid rows.

Input:

```text
abc|def|ghi|
```

Command:

```bash
printf 'abc|def|ghi|' |
  rkg 'd.rs("|")'
```

Output:

```text
abc
def
ghi
```

#### `ofs(sep)`

Changes the separator used when cells are joined for each output row.

Input:

```text
abc
def
```

Command:

```bash
printf 'abc\ndef\n' |
  rkg 'd.ofs("|")'
```

Output:

```text
a|b|c
d|e|f
```

#### `ors(sep)`

Changes the separator used when output rows are joined together.

Input:

```text
abc
def
```

Command:

```bash
printf 'abc\ndef\n' |
  rkg 'd.ors("---\n")'
```

Output:

```text
abc---
def---
```

#### `t()` / `transpose()`

Swaps rows and columns in the grid.

Input:

```text
abc
def
ghi
```

Command:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'd.t()'
```

Output:

```text
adg
beh
cfi
```

#### `rt("r"|"l"|"180")` / `rotate(...)`

Rotates the grid 90 degrees clockwise.

Input:

```text
abc
def
ghi
```

Command:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'd.rt("r")'
```

Output:

```text
gda
heb
ifc
```

Rotates the grid by 180 degrees.

Input:

```text
abc
def
ghi
```

Command:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'd.rt("180")'
```

Output:

```text
ihg
fed
cba
```

#### `m(from, ray, put)`

Marks all reachable cells along the specified ray directions from the source cell.

Input:

```text
.....
..K..
.....
```

Command:

```bash
printf '.....\n..K..\n.....\n' |
  rkg 'd.m("K","rook","*")'
```

Output:

```text
..*..
**K**
..*..
```

#### `m(from, through_re, to, put)`

Marks only the matching middle cells when they are sandwiched between `from` and `to`.

Input:

```text
.....
.XOOX
.....
```

Command:

```bash
printf '.....\n.XOOX\n.....\n' |
  rkg 'd.m("X","O","X","*")'
```

Output:

```text
.....
.X**X
.....
```

### Multiple statements

Runs each statement against the original stdin, so later statements do not receive earlier output.

Input:

```text
A 10,20
B 7,8
```

Command:

```bash
printf 'A 10,20\nB 7,8\n' |
  rkg --print-all 'r.x(2,",").g(1,s(2)); r.n(1)'
```

Output:

```text
A 30
B 15
---
1 A 10,20
2 B 7,8
```

## Notes

- `fs` is treated as a regex for record mode, similar to AWK `FS`
- CSV quoting is **not** implemented; this prototype is regex-split based
- `;` resets to the original stdin; it does **not** pass the previous statement result to the next one
- grid mode defaults to character cells; `d.fs(",")` switches to separated cells
