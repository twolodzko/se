# se

## Syntax

`se` is a command-line utility similar to [`sed`]. It can be called as

```text
se [OPTIONS] <SCRIPT> [FILE]...
```

where `<SCRIPT>` contains instructions (separated by `;` or `.`, see [below](#multiple-instructions)) of the form

```text
[address][command]
```

The `command` is executed for each line from the input the `[FILE]`'s that matches the `address`.
While the syntax of the instruction is similar to `sed`'s, it is not the same and not meant to be so.
Rather than being a replacement, it is `sed`'s simplified cousin, using modernized syntax,
and written in Rust.

Same as `sed`, it can be used for string search and replace in files.

## How it works?

`se` works in a [similar way as `sed`]:

> `sed` maintains two data buffers: the active *pattern* space, and the auxiliary *hold* space.
> Both are initially empty.
>
> `sed` operates by performing the following cycle on each line of input: first, `sed` reads one line from
> the input stream, removes any trailing newline, and places it in the pattern space.
> Then commands are executed; each command can have an address associated to it: addresses are a kind
> of condition code, and a command is only executed if the condition is verified before the command
> is to be executed.
>
> When the end of the script is reached [...] the contents of
> pattern space are printed out to the output stream, adding back the trailing newline if
> it was removed. Then the next cycle starts for the next input line.

## Addresses

* Number like `1` or `278` points to a specific line. Line numbers start at 1.
* `$` never matches, so `5-$` (or `5-`) means a left-open interval.
  Commands with the `$` address would run unconditionally after processing all the lines,
  even after early stopping using `q`.
* `/regex/` matches the lines that match the regular expression specified between `/.../`.
  Regular expressions can be used as bounds of the ranges.
* `//` means that any line would match. If no address is given, this is the default.
* `^regex$` can be used instead of `/^regex$/` when matching the whole line.
  Because in other cases regular expressions are delimited with `/.../`,
  even when not using slashes `\/` would be interpreted a escaped slash.
  `^$` would match empty lines.
* `?` matches the lines where the following substitution could be applied.
  It is a syntactic sugar for writing `?s/src/dst/` instead of `/src/ s/src/dst/`.
* `!` before the address negates it, e.g. `!1` means all the lines except the first.
* Addresses can be enclosed with brackets `(addr)`.

Addresses can be combined:

* `start-end` is an inclusive range. For example, `1-5` includes lines
  between `1` and `5`. `-5` is equivalent to `1-5`. `1-` or `1-$` means all the lines
  from `1` to the final line. `/foo/-/bar/` is a range of lines where the first line
  contains the word "foo" and the last line the word "bar".
* `start~step` matches every `step`-th line since line number `start`.
* `start+window` matches `start` line and next `window` number of lines after it.
* `addr1, addr2, ..., addrN` matches any of the addresses.
* `addr1 & addr2 & ... & addrN` matches only if all of the addresses matched.

`/a/ & /b/-/c/, /d/` is equivalent to `(/a/ & (/b/-/c/)), /d/` because of the `-` has higher
precedence than `&`, and `&` then `,`.

## Commands

### Printing

* `p` – print the content of the pattern space as-is followed by a newline character.
* `P` – same as above, but without the newline.
* `=` – print the line number.
* `\n`, `\t`, `\x0A`, `\uA005` – print special characters, escaping a character recognized
  as command like `\p` would print the character "p".
* `"string"` or `'string'` – print the `string`. The `string` can contain special escape
  characters like `\n` or `\t`.

### Editing

* `s/src/dst/[limit]` – use regular expression to replace `src` with `dst` in the pattern space.
  If there's nothing to substitute, it has no effect. `limit` is a number of matches to replace.
* `k N-M` – keep the characters from the `N-M` range (inclusive). `M` means `M`th character,
  `-M` is an left-open interval (same as `1-M`), `N-` is an right-open interval.
* `z` – empty the content of pattern space. It is the same as `s/.*//`, but is more efficient.
* `l`, `L` – escape characters with Rust's [std::char::escape_default] and unescape them.
* `u`, `U` – URL encode and decode characters.
* `t`, `T` – HTML-escape and unescape characters.
* `b`, `B` – convert characters to base64 and back.

### Manipulating memory

* `c` - set pattern space to the original, unprocessed line.
* `h` – hold the content of the pattern space to the hold space.
* `g` – get the content of the hold space to the pattern space.
* `x` – exchange the content of the pattern space with content of the hold space.
* `j` – push the content of the hold space at the back of the pattern space
  using a newline character as separator.
* `J` – same as above, but without the separator.

### Special actions

* `r [num]` – read `num` lines (1 by default) and append them to pattern space
  using newline as a separator.
* `R` – read new line and replace pattern space content with it. If it cannot read the new line,
  it send the break signal (same as `.`).
* `d` – clear the content of the pattern space and immediately start processing next line.
* `e` – execute the content of the pattern space as a shell command. Save the stdout output
  of the command to pattern space. If the command returned with non-zero error code,
  stop and return the error code.
* `q [code]` – exit with the `code` exit code (0 by default).

## Multiple instructions

When script contains multiple instructions, they can be delimited with `;` or `.`.

* `;` is used for chaining instructions. After processing the instruction,
  the pattern space would be processed using the following instruction.
* `.` marks the final instruction. If the address of the instruction would positively match,
  the processing of the line would stop after running the command,
  all the following instructions would be skipped.
  In a way, `.` works like the command `d`, but it does not clear the pattern space.

For example, the script

```text
/sed/ ">> " p .
      "   " p
```

when applied to this README would print it's content prepending each line containing the word "sed"
with ">> " and every other line (no address) with spaces. If `;` was used instead of `.`, the
lines containing the word "sed" would be printed twice, because of matching addresses in the both instructions.

## Differences from `sed`

* Using [Rust's Regex] regular expression syntax, including the syntax for flags
  e.g. `/(?i)regex/` is used instead of `/regex/i`. The flags can be used in
  matches as well as substitutions. With `(?x)` flag it is possible to write regular
  expressions in [verbose mode], which can include comments.
* When using `\N` for substitutions, N could be a name of a named group (but to avoid ambiguity best use `\{N}`).
* Not using the command groups syntax `{ cmd1 ; cmd2 ; ... }`,
  but instead reading commands directly e.g. `=p` (actually `=\np`, see [above](#commands)) is equivalent to `{ = ; p }` in `sed`.
* Only a subset of `sed` commands is supported and they can behave differently.
* Instead of `a string`, use `p"string"` to print the string after
  printing the line, same applies to `sed`s `i`.
* `sed` by default prints all the lines unless explicitly deleted.
  To achieve this behavior use `-a` (`--all`) flag to print all the lines.
* In `sed` the block after `$` runs on the final line, in `se`
  it is an instruction set that runs unconditionally on the program stop.
* `se` by default replaces all matches (like `s/src/dst/g` in sed) so it does not use the /g flag.
* `s/src/dst/` does pure substitution. It returns unchanged lines on no match, unlike `sed` which skips such lines.
  To imitate `sed`s execution flow conditional on substitutions, use `?` (see [addresses](#addresses)).

|      `sed`       |       `se`          |
|------------------|---------------------|
| `=`              | `=\np`              |
| `i text`         | `p "text\n"`        |
| `a text`         | `"text\n" p`        |
| `{c1 ; c2 ; c3}` | `c1 c2 c3`          |
| `s/src/dst/`     | `s/src/dst/1`       |
| `s/src/dst/g`    | `s/src/dst/`        |
| `s/src/dst/flag` | `s/(?flag)src/dst/` |
| `s/(src)/\1/g`   | `s/(src)/\1/`       |
| `s/(src)/&/g`    | `s/(src)/\0/`       |
| `1,5p`           | `1-5p`              |
| `$p`             | `$p`                |

## `se` vs other command line utilities

|    other                             |   `se`                           |
|--------------------------------------|----------------------------------|
| `cat README.md`                      | `se 'p' README.md`               |
| `tac README.md`                      | `se '!1 j ; $p ; h' README.md`   |
| `cat -n README.md`                   | `se '=\tp' README.md`            |
| `sed -E 's/(sed)/_\1_/g' README.md`  | `se 's/(sed)/_\1_/p' README.md`  |
| `sed -n 's/a/#/p' README.md`         | `se '?s/a/#/1p' README.md`       |
| `sed 's/sed/###/g' README.md`        | `se -a 's/sed/###/' README.md`   |
| `head -n 5 README.md`                | `se '-5 p . q' README.md`        |
| `head -n 5 README.md`                | `se 'r4 p q' README.md`          |
| `cut -c '3-7' README.md`             | `se 'k3-7 p' README.md`\*        |
| `grep 'sed' README.md`               | `se '/sed/ p' README.md`         |
| `grep -c 'sed' README.md`            | `se -c '/sed/' README.md`        |
| `wc -l README.md`                    | `se -c '' README.md`             |
| `wc -l README.md`                    | `se '$=' README.md`              |

\* – but `se` understands unicode.

[`sed`]: https://www.gnu.org/software/sed/manual/sed.html
[Rust's Regex]: https://docs.rs/regex/latest/regex/
[verbose mode]: https://docs.rs/regex/latest/regex/?search=verbose#example-verbose-mode
[std::char::escape_default]: https://doc.rust-lang.org/std/primitive.char.html#method.escape_default
[similar way as `sed`]: https://www.gnu.org/software/sed/manual/sed.html#Execution-Cycle
