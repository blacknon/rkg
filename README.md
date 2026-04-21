rkg
===

**`r`ecord** + **`k`nit** + **`g`rid**

## Description

`rkg` is a one-liner oriented record/grid processor for text reshaping work.

It combines record-style operations for delimited text with grid-style operations for line-based patterns, so you can select, replace, reshape, transpose, rotate, and otherwise transform structured text from a compact command-line syntax.

For shell-friendly one-liners, the DSL should prefer a small punctuation set built
around `.`, `:`, `,`, `;`, and `=` so that common expressions stay readable in
bash/zsh without leaning too heavily on `"` or `()`.

### Features

- `r.` / `rec.` for record mode
- `g.` / `grid.` for grid mode
- method chaining with `.`
- pipeline chaining with `|`
- statement reset with `;`
- AWK-like separators: `fs`, `rs`, `ofs`, `ors`
- `-F` / `--field-separator` for AWK-like initial field separator override
- `-R` / `--record-separator`, `-O` / `--output-field-separator`, `-N` / `--output-record-separator` for initial separator overrides
- field selection, replace, explode, implode, groupby, reshape, flatten
- transpose, rotate, and ray/pattern mark operations for grid input

## Install

### Cargo Install

```bash
cargo install rkg
```

### Build From Source

```bash
cargo build --release
```

## Usage

### Command

```bash
$ rkg --help
Record/grid DSL processor
```

### Quick Start

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  cargo run -- -F '[,:;]' 'r.p(1,2,3).ofs("|")'
```

Shorthand:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  cargo run -- -F '[,:;]' 'r.p:1,2,3.ofs=|'
```

Existing commands:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  awk -F'[,:;]' '{print $1 "|" $2 "|" $3}'
```

### DSL Shape

```text
r.fs(",").x(2,";").g(1,s(2)).ofs(",");
g.t().rt("r").m("K","orth","*")
```

Shorthand forms are also supported for shell-friendly one-liners:

```text
mode.method:arg1,arg2.setting=value;mode.method:arg
mode.method | mode.method
g.t.rt:r
```

- `|` pipes the previous stage output into the next stage when the right side starts with `r.`/`g.`
- `method(...)` is the classic call form
- `method:arg1,arg2` is shorthand for `method(arg1,arg2)`
- `method=value` is shorthand for single-argument config-style calls like `ofs("|")`
- bare `method` is shorthand for zero-argument calls like `t()`
- `;` resets evaluation to the original stdin for the next statement group
- only the last statement is printed by default
- `--print-all` prints all statement results separated by `---`
- `-F` sets the initial record-mode `fs` before the DSL runs, and accepts regex patterns
- `-R`, `-O`, and `-N` set initial `rs`, `ofs`, and `ors` before the DSL runs
- if `EXPR` is omitted, `rkg` defaults to record-mode passthrough with the initial CLI separators applied
- when the shell would treat a character specially, quote the whole DSL as one argument

### Quick examples

#### Record functions

##### `-F re` / `--field-separator re`

Sets the initial record-mode field separator from the CLI before any DSL method runs.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  rkg -F '[,:;]' 'r.p:1,2,3.ofs=|'
```

Existing commands:

```bash
printf 'A,10;tokyo\nB:20;osaka\n' |
  awk -F'[,:;]' '{OFS="|"}{print $1,$2,$3}'
```

</details>

##### Shorthand pipeline chaining

Passes shorthand output from one stage directly into the next stage with `|`.

<details>
<summary>Example</summary>

Input:

```text
A 10
B 20
```

Command:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.p:1,2.ofs=- | g.t'
```

Existing commands:

```bash
printf 'A 10\nB 20\n' |
  awk '{s=$1 "-" $2; for(i=1;i<=length(s);i++) col[i]=col[i] substr(s,i,1)} END{for(i=1; i in col; i++) print col[i]}'
```

Output:

```text
AB
--
12
00
```

</details>

##### `fs(re)`

Splits each record with the given regex instead of the default whitespace separator.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A,10\nB,20\n' |
  rkg 'r.fs=,'
```

Option only:

```bash
printf 'A,10\nB,20\n' |
  rkg -F,
```

Existing commands:

```bash
printf 'A,10\nB,20\n' |
  awk -F',' '{print $1, $2}'
```

Output:

```text
A 10
B 20
```

</details>

##### `rs(sep)`

Treats the given separator as the boundary between input records.

<details>
<summary>Example</summary>

Input:

```text
A 10|B 20|
```

Command:

```bash
printf 'A 10|B 20|' |
  rkg 'r.rs("|")'
```

Shorthand:

```bash
printf 'A 10|B 20|' |
  rkg 'r.rs=|'
```

Option only:

```bash
printf 'A 10|B 20|' |
  rkg -R'|'
```

Existing commands:

```bash
printf 'A 10|B 20|' |
  awk 'BEGIN{RS="\\|"} NF{print $0}'
```

Output:

```text
A 10
B 20
```

</details>

## Reference

### Record functions

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

### Aggregators

- `s(col)` / `sum(col)`
- `c()` / `count()`
- `mn(col)` / `min(col)`
- `mx(col)` / `max(col)`
- `a(col)` / `avg(col)`
- `med(col)` / `median(col)`

### Grid functions

- `fs(sep)` optional cell separator; default is character grid
- `rs(sep)` / `ofs(sep)` / `ors(sep)`
- `t()` / `transpose()`
- `rt("r"|"l"|"180")` / `rotate(...)`
- `m(from, ray, put)` marks along a ray (`orth`, `diag`, `alldir`, `8`)
- `m(from, through_re, to, put)` 8-direction pattern mark, useful for reversi-like scans

### Shorthand syntax

- `r.p:1,3.ofs=|` is equivalent to `r.p(1,3).ofs("|")`
- `r.g:1,s:2` is equivalent to `r.g(1,s(2))`
- `g.t.rt:r` is equivalent to `g.t().rt("r")`
- shorthand is most useful for simple one-liners; regular `()` calls remain available for anything that needs clearer quoting
- the `Existing commands` snippets below are example-specific equivalents built from common shell tools, not drop-in general replacements for the full DSL

## Examples

### Record functions

#### `ofs(sep)` / output field separator

Changes the separator used when fields are joined for output.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.ofs=,'
```

Option only:

```bash
printf 'A 10\nB 20\n' |
  rkg -O,
```


Existing commands:

```bash
printf 'A 10\nB 20\n' |
  awk '{$1=$1; OFS=","; print}'
```

Output:

```text
A,10
B,20
```

</details>

#### `ors(sep)` / output record separator

Changes the separator used when output records are joined together.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.ors=|'
```

Option only:

```bash
printf 'A 10\nB 20\n' |
  rkg -N'|'
```

Existing commands:

```bash
printf 'A 10\nB 20\n' |
  awk 'BEGIN{ORS="|"} {print $0}'
```

Output:

```text
A 10|B 20|
```

</details>

#### `p(...)` / `select(...)`

Keeps only the requested fields and removes the rest.

<details>
<summary>Example</summary>

Input:

```text
A 10 tokyo
B 20 osaka
```

Command:

```bash
printf 'A 10 tokyo\nB 20 osaka\n' |
  rkg 'r.p(1,3)'
```

Shorthand:

```bash
printf 'A 10 tokyo\nB 20 osaka\n' |
  rkg 'r.p:1,3'
```

Existing commands:

```bash
printf 'A 10 tokyo\nB 20 osaka\n' |
  awk '{print $1, $3}'
```

Output:

```text
A tokyo
B osaka
```

</details>

#### `sb(re, rep)` / `replace(re, rep)`

Replaces text matching the regex in every cell.

<details>
<summary>Example</summary>

Input:

```text
A-10
B-20
```

Command:

```bash
printf 'A-10\nB-20\n' |
  rkg 'r.sb("[0-9]","X")'
```

Shorthand:

```bash
printf 'A-10\nB-20\n' |
  rkg 'r.sb:[0-9],X'
```

Existing commands:

```bash
printf 'A-10\nB-20\n' |
  sed -E 's/[0-9]/X/g'
```

Output:

```text
A-XX
B-XX
```

</details>

#### `n(start_or_AZ)` / `enum(...)`

Adds a numeric counter column to the front of each row.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10\nB 20\n' |
  rkg 'r.n:1'
```

Existing commands:

```bash
printf 'A 10\nB 20\n' |
  awk '{print NR, $0}'
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

Shorthand:

```bash
printf 'A 10\nB 20\nC 30\n' |
  rkg 'r.n:A-Z'
```

Existing commands:

```bash
printf 'A 10\nB 20\nC 30\n' |
  awk '{printf "%c %s\n", 64 + NR, $0}'
```

Output:

```text
A A 10
B B 20
C C 30
```

</details>

#### `x(col, sep)` / `explode(...)`

Splits one field into multiple rows while keeping the other columns as-is.

<details>
<summary>Example</summary>

Input:

```text
A,10;20;30
B,7;8
```

Command:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg -F, -O, 'r.x(2,";")'
```

Shorthand:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg -F, -O, 'r.x:2,";"'
```

Existing commands:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  awk -F',' '{n=split($2, a, ";"); for (i=1; i<=n; i++) print $1 "," a[i]}'
```

Output:

```text
A,10
A,20
A,30
B,7
B,8
```

</details>

#### `i(key_col, val_col, join_sep?)` / `implode(...)`

Merges rows with the same key by joining one value column into a single field.

<details>
<summary>Example</summary>

Input:

```text
A 10
A 20
A 30
B 7
B 8
B 9
C 100
C 200
```

Command:

```bash
printf 'A 10\nA 20\nA 30\nB 7\nB 8\nB 9\nC 100\nC 200\n' |
  rkg 'r.i(1,2,",")'
```

Shorthand:

```bash
printf 'A 10\nA 20\nA 30\nB 7\nB 8\nB 9\nC 100\nC 200\n' |
  rkg 'r.i:1,2,","'
```

Existing commands:

```bash
printf 'A 10\nA 20\nA 30\nB 7\nB 8\nB 9\nC 100\nC 200\n' |
  awk '!seen[$1]++{keys[++n]=$1} {vals[$1]=vals[$1] ? vals[$1] "," $2 : $2} END {for (i=1; i<=n; i++) print keys[i], vals[keys[i]]}'
```

Output:

```text
A 10,20,30
B 7,8,9
C 100,200
```

</details>

#### `g(key_col, agg...)` / `groupby(...)`

Groups rows by the key column and emits aggregate results per group.

<details>
<summary>Example</summary>

Input:

```text
A,10;20;30
B,7;8
```

Command:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg -F, -O, 'r.x(2,";").g(1,s(2))'
```

Shorthand:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  rkg  -F, -O, 'r.x:2,";".g:1,s:2'
```

Existing commands:

```bash
printf 'A,10;20;30\nB,7;8\n' |
  awk -F'[,;]' '{if (!seen[$1]++) keys[++n]=$1; for (i=2; i<=NF; i++) sum[$1]+=$i} END {for (i=1; i<=n; i++) print keys[i] "," sum[keys[i]]}'
```

Output:

```text
A,60
B,15
```

</details>

#### `sh("w2l", ...)` / `reshape(...)`

Turns wide columns into repeated long-form rows.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'name math eng\nA 80 90\nB 70 85\n' |
  rkg 'r.sh:w2l,2'
```

Existing commands:

```bash
printf 'name math eng\nA 80 90\nB 70 85\n' |
  awk 'NR==1{for (i=2; i<=NF; i++) h[i]=$i; next} {for (i=2; i<=NF; i++) print $1, h[i], $i}'
```

Output:

```text
A math 80
A eng 90
B math 70
B eng 85
```

</details>

#### `sh("l2w", ...)` / `reshape(...)`

Turns repeated long-form rows back into a wide table.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A math 80\nA eng 90\nB math 70\nB eng 85\n' |
  rkg 'r.sh:l2w,2,3'
```

Existing commands:

```bash
printf 'A math 80\nA eng 90\nB math 70\nB eng 85\n' |
  sort -k2,2 -k1,1 |
  awk 'BEGIN{OFS=" "} !c[$2]++{col[++m]=$2} !r[$1]++{row[++n]=$1} {a[$1,$2]=$3} END{printf "key"; for(i=1;i<=m;i++) printf OFS col[i]; print ""; for(j=1;j<=n;j++){printf row[j]; for(i=1;i<=m;i++) printf OFS a[row[j],col[i]]; print ""}}'
```

Output:

```text
key eng math
A 90 80
B 85 70
```

</details>

#### `f(template?)` / `flatten(...)`

Renders each data row as a single string using the header names in the template.

<details>
<summary>Example</summary>

Input:

```text
name age
alice 20
bob 30
carol 25
dave 41
```

Command:

```bash
printf 'name age\nalice 20\nbob 30\ncarol 25\ndave 41\n' |
  rkg 'r.f("{name}:{age}")'
```

Shorthand:

```bash
printf 'name age\nalice 20\nbob 30\ncarol 25\ndave 41\n' |
  rkg 'r.f:"{name}:{age}"'
```

Existing commands:

```bash
printf 'name age\nalice 20\nbob 30\ncarol 25\ndave 41\n' |
  awk 'NR>1 {print $1 ":" $2}'
```

Output:

```text
alice:20
bob:30
carol:25
dave:41
```

</details>

### Aggregators

#### `s(col)` / `sum(col)`

Sums the numeric values in the target column for each group.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g:1,s:2'
```

Existing commands:

```bash
printf 'A 10\nA 20\nB 7\n' |
  awk '{c[$1]+=$2} END {for (k in c) print k, c[k]}' | sort
</details>

# or, with datamash:
printf 'A 10\nA 20\nB 7\n' |
  datamash -s -g 1 sum 2
```

Output:

```text
A 30
B 7
```

#### `c()` / `count()`

Counts how many rows belong to each group.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10\nA 20\nB 7\n' |
  rkg 'r.g:1,c'
```

Existing commands:

```bash
printf 'A 10\nA 20\nB 7\n' |
  awk '{print $1}' | uniq -c | awk '{print $2, $1}'
</details>

# or, with datamash:
printf 'A 10\nA 20\nB 7\n' |
  datamash -s -g 1 count 1
```

Output:

```text
A 2
B 1
```

#### `mn(col)` / `min(col)`

Takes the smallest numeric value in the target column for each group.

<details>
<summary>Example</summary>

Input:

```text
A 10
A 20
A 15
B 7
B 12
C 3
C 9
```

Command:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g(1,mn(2))'
```

Shorthand:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g:1,mn:2'
```

Existing commands:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  sort -k1,1 -k2,2n | awk '!a[$1]++'
</details>

# or, with datamash:
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  datamash -s -g 1 min 2
```

Output:

```text
A 10
B 7
C 3
```

#### `mx(col)` / `max(col)`

Takes the largest numeric value in the target column for each group.

<details>
<summary>Example</summary>

Input:

```text
A 10
A 20
A 15
B 7
B 12
C 3
C 9
```

Command:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g(1,mx(2))'
```

Shorthand:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g:1,mx:2'
```

Existing commands:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  sort -k1,1 -k2,2nr | awk '!a[$1]++'
</details>

# or, with datamash:
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  datamash -s -g 1 max 2
```

Output:

```text
A 20
B 12
C 9
```

#### `a(col)` / `avg(col)`

Computes the average numeric value in the target column for each group.

<details>
<summary>Example</summary>

Input:

```text
A 10
A 20
A 15
B 7
B 12
C 3
C 9
```

Command:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g(1,a(2))'
```

Shorthand:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g:1,a:2'
```

Existing commands:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  awk '{sum[$1]+=$2; cnt[$1]++} END {for (k in sum) print k, sum[k] / cnt[k]}' |
  sort
</details>

# or, with datamash:
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  datamash -s -g 1 mean 2
```

Output:

```text
A 15
B 9.5
C 6
```

#### `med(col)` / `median(col)`

Computes the median numeric value in the target column for each group.

<details>
<summary>Example</summary>

Input:

```text
A 10
A 20
A 15
B 7
B 12
C 3
C 9
```

Command:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g(1,med(2))'
```

Shorthand:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  rkg 'r.g:1,med:2'
```

Existing commands:

```bash
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  awk '{a[$1][++n[$1]]=$2} END {for (k in a) {asort(a[k]); print k, n[k]%2 ? a[k][(n[k]+1)/2] : (a[k][n[k]/2]+a[k][n[k]/2+1])/2}}' |
  sort
</details>

# or, with datamash:
printf 'A 10\nA 20\nA 15\nB 7\nB 12\nC 3\nC 9\n' |
  datamash -s -g 1 median 2
```

Output:

```text
A 15
B 9.5
C 6
```

### Grid functions

#### `fs(sep)` / cell separator

Treats each input row as separator-delimited cells instead of a character grid.

<details>
<summary>Example</summary>

Input:

```text
a,b,c
d,e,f
```

Command:

```bash
printf 'a,b,c\nd,e,f\n' |
  rkg -O'|' 'g.fs(",")'
```

Shorthand:

```bash
printf 'a,b,c\nd,e,f\n' |
  rkg -O'|' 'g.fs=,'
```

Existing commands:

```bash
printf 'a,b,c\nd,e,f\n' |
  awk -F',' '{print $1 "|" $2 "|" $3}'
```

Output:

```text
a|b|c
d|e|f
```

</details>

#### `rs(sep)` / record separator

Treats the given separator as the boundary between grid rows.

<details>
<summary>Example</summary>

Input:

```text
abc|def|ghi|
```

Command:

```bash
printf 'abc|def|ghi|' |
  rkg 'g.rs("|")'
```

Shorthand:

```bash
printf 'abc|def|ghi|' |
  rkg 'g.rs=|'
```

Option only:

```bash
printf 'abc|def|ghi|' |
  rkg -R'|'
```

Existing commands:

```bash
printf 'abc|def|ghi|' |
  awk 'BEGIN{RS="\\|"} NF {print $0}'
```

Output:

```text
abc
def
ghi
```

</details>

#### `ofs(sep)` / output field separator

Changes the separator used when cells are joined for each output row.

<details>
<summary>Example</summary>

Input:

```text
abc
def
```

Command:

```bash
printf 'abc\ndef\n' |
  rkg 'g.ofs("|")'
```

Shorthand:

```bash
printf 'abc\ndef\n' |
  rkg 'g.ofs=|'
```

Option only:

```bash
printf 'abc\ndef\n' |
  rkg -O'|'
```

Existing commands:

```bash
printf 'abc\ndef\n' |
  sed 's/./&|/g; s/|$//'
```

Output:

```text
a|b|c
d|e|f
```

</details>

#### `ors(sep)` / output record separator

Changes the separator used when output rows are joined together.

<details>
<summary>Example</summary>

Input:

```text
abc
def
```

Command:

```bash
printf 'abc\ndef\n' |
  rkg 'g.ors("---\n")'
```

Shorthand:

```bash
printf 'abc\ndef\n' |
  rkg 'g.ors="---\n"'
```

Option only:

```bash
printf 'abc\ndef\n' |
  rkg -N'---\n'
```

Existing commands:

```bash
printf 'abc\ndef\n' |
  awk 'BEGIN{ORS="---\n"} {print $0}'
```

Output:

```text
abc---
def---
```

</details>

#### `t()` / `transpose()`

Swaps rows and columns in the grid.

<details>
<summary>Example</summary>

Input:

```text
abc
def
ghi
```

Command:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.t()'
```

Shorthand:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.t'
```

Existing commands:

```bash
printf 'abc\ndef\nghi\n' |
  awk '{for (i=1; i<=length($0); i++) col[i]=col[i] substr($0, i, 1)} END {for (i=1; i in col; i++) print col[i]}'
```

Output:

```text
adg
beh
cfi
```

</details>

#### `rt("r"|"l"|"180")` / `rotate(...)`

Rotates the grid 90 degrees clockwise.

<details>
<summary>Example</summary>

Input:

```text
abc
def
ghi
```

Command:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.rt("r")'
```

Shorthand:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.rt:r'
```

Existing commands:

```bash
printf 'abc\ndef\nghi\n' |
  awk '{rows[NR]=$0; if (length($0)>w) w=length($0)} END {for (i=1; i<=w; i++) {out=""; for (j=NR; j>=1; j--) out = out substr(rows[j], i, 1); print out}}'
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
  rkg 'g.rt("180")'
```

Shorthand:

```bash
printf 'abc\ndef\nghi\n' |
  rkg 'g.rt=180'
```

Existing commands:

```bash
printf 'abc\ndef\nghi\n' |
  awk '{rows[NR]=$0} END {for (i=NR; i>=1; i--) {out=""; for (j=length(rows[i]); j>=1; j--) out = out substr(rows[i], j, 1); print out}}'
```

Output:

```text
ihg
fed
cba
```

</details>

#### `m(from, ray, put)` / `mark(from, ray, put)`

Marks all reachable cells along the specified ray directions from the source cell.

<details>
<summary>Example</summary>

Input:

```text
.....
..K..
.....
```

Command:

```bash
printf '.....\n..K..\n.....\n' |
  rkg 'g.m("K","orth","*")'
```

Shorthand:

```bash
printf '.....\n..K..\n.....\n' |
  rkg 'g.m:K,orth,*'
```

Existing commands:

```bash
printf '.....\n..K..\n.....\n' |
  awk '{rows[NR]=$0; if ((p=index($0,"K"))>0) {ky=NR; kx=p}} END {for (y=1; y<=NR; y++) {out=""; for (x=1; x<=length(rows[y]); x++) {c=substr(rows[y], x, 1); if ((y==ky || x==kx) && !(y==ky && x==kx) && c==".") c="*"; out=out c} print out}}'
```

Output:

```text
..*..
**K**
..*..
```

</details>

#### `m(from, through_re, to, put)` / `mark(from, through_re, to, put)`

Marks only the matching middle cells when they are sandwiched between `from` and `to`.

<details>
<summary>Example</summary>

Input:

```text
.....
.XOOX
.....
```

Command:

```bash
printf '.....\n.XOOX\n.....\n' |
  rkg 'g.m("X","O","X","*")'
```

Shorthand:

```bash
printf '.....\n.XOOX\n.....\n' |
  rkg 'g.m:X,O,X,*'
```

Existing commands:

```bash
printf '.....\n.XOOX\n.....\n' |
  sed 's/XOOX/X**X/'
```

Output:

```text
.....
.X**X
.....
```

</details>

### Multiple statements

Runs each statement against the original stdin, so later statements do not receive earlier output.

<details>
<summary>Example</summary>

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

Shorthand:

```bash
printf 'A 10,20\nB 7,8\n' |
  rkg --print-all 'r.x:2,",".g:1,s:2; r.n:1'
```

Existing commands:

```bash
{ printf 'A 10,20\nB 7,8\n' | awk '{split($2, a, ","); print $1, a[1] + a[2]}'; printf '%s\n' '---'; printf 'A 10,20\nB 7,8\n' | awk '{print NR, $0}'; }
```

Output:

```text
A 30
B 15
---
1 A 10,20
2 B 7,8
```

</details>

## Notes

- `fs` is treated as a regex for record mode, similar to AWK `FS`
- CSV quoting is **not** implemented; this prototype is regex-split based
- `;` resets to the original stdin; it does **not** pass the previous statement result to the next one
- grid mode defaults to character cells; `g.fs(",")` switches to separated cells
