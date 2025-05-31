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
* `1-5` an inclusive range of the lines between `1` and `5`.
  `-5` is equivalent to `1-5`.
  `1-` or `1-$` means all the lines from `1` to the final line.
* `//` or no address specified means that all the lines would match.
  If no address is given, this is the default.
* `$` matches the final line, so `5-$` (or `5-`) means a left-open interval.
  Commands in the block after `$` would run unconditionally, after processing the files,
  even after early stopping using `q`.
* `/regex/` matches the lines that match the regular expression specified between `/.../`.
  Regular expressions can be used as bounds of the ranges.
* `^regex$` can be used instead of `/^regex$/` when matching the whole line.
  Because in other cases regular expressions are delimited with `/.../`,
  even when not using slashes `\/` would be interpreted a escaped slash.
* `addr1,addr2,...,addrN` matches any of the addresses.
* `!` after the address negates it, e.g. `1!` means all the lines except the first.
* Addresses can be enclosed with brackets `(addr)`. It can be used together with negation,
  e.g. `(1,2,3)!` is equivalent to matching the `4-` range.

## Commands

* `p` – print the content of the pattern space as-is followed by a newline character.
* `P` – same as above, but without the newline.
* `l` – print the content of the pattern space after escaping the characters with Rust's
  [std::char::escape_default].
* `=` – print the line number.
* `n`, `t` – print newline or tab character.
* `s/src/dst/[limit]` – use regular expression to replace `src` with `dst` in the pattern space.
* `k N-M` – keep the characters from the `N-M` range (inclusive). `M` means `M`th character,
  `-M` is an left-open interval (same as `1-M`), `N-` is an right-open interval.
* `h` – hold the content of the pattern space to the hold space.
* `g` – get the content of the hold space to the pattern space.
* `x` – exchange the content of the pattern space with content of the hold space.
* `j` – push the content of the hold space at the back of the pattern space
        using a newline character as separator.
* `J` – same as above, but without the separator.
* `r [num]` – read `num` lines (1 by default) and append them to pattern space
        using newline as a separator.
* `z` – empty the content of pattern space. It is the same as `s/.*//`, but is more efficient.
* `d` – clear the content of the pattern space and immediately start processing next line.
* `"string"` or `'string'` – print the `string`. The `string` can contain special escape
  characters like `\n` or `\t`.
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
* Using `$N` for substitutions instead of `\N`.
* Not using the command groups syntax `{ cmd1 ; cmd2 ; ... }`,
  but instead reading commands directly e.g. `=p` (actually `="\n"p`, see above) is equivalent to `{ = ; p }` in `sed`.
* Only a subset of `sed` commands is supported and they can behave differently.
* Instead of `a string`, use `p"string"` to print the string after
  printing the line, same applies to `sed`s `i`.
* No support for branching.
* `sed` by default prints all the lines unless explicitly deleted.
  To achieve this behavior use `-a` (`--all`) flag to print all the lines.
* In `sed` the block after `$` runs on the final line, in `se`
  it is an instruction set that runs unconditionally on the program stop.
* `se` uses `s/src/dst/g` as a default rather than `s/src/dst/1` as `sed` does.
* `s/src/dst/` does pure substitution. It returns unchanged lines on no match, unlike `sed` which skips such lines.

|      `sed`       |       `se`          |
|------------------|---------------------|
| `=`              | `="\n"p`            |
| `i text`         | `p "text\n"`        |
| `a text`         | `"text\n" p`        |
| `{c1 ; c2 ; c3}` | `c1 c2 c3`          |
| `s/src/dst/`     | `s/src/dst/1`       |
| `s/src/dst/g`    | `s/src/dst/`        |
| `s/src/dst/flag` | `s/(?flag)src/dst/` |
| `s/(src)/\1/g`   | `s/(src)/$1/`       |
| `s/(src)/&/g`    | `s/(src)/$0/`       |
| `1,5p`           | `1-5p`              |
| `$p`             | `$p`                |

## `se` vs other command line utilities

|    other                       |   `se`                          |
|--------------------------------|---------------------------------|
| `cat README.md`                | `se 'p' README.md`              |
| `cat -n README.md`             | `se '= "\t" p' README.md`       |
| `sed 's/sed/###/g' README.md`  | `se -a 's/sed/###/' README.md`  |
| `head -n 5 README.md`          | `se '-5 p . q' README.md`       |
| `head -n 5 README.md`          | `se 'r4 p q' README.md`         |
| `cut -c '3-7' README.md`       | `se 'k3-7 p' README.md`\*       |
| `grep 'sed' README.md`         | `se '/sed/ p' README.md`        |
| `grep -c 'sed' README.md`      | `se -c '/sed/' README.md`       |
| `wc -l README.md`              | `se -c '' README.md`            |
| `wc -l README.md`              | `se '$=' README.md`             |

\* – but `se` understands unicode.

## Grammar

```text
Location       = [1-9][0-9]*
Regex          = '/' [^/]* '/'
WholeLine      = '^' [^$]* '$'
AddressAtom    = '$' | Location | Regex | WholeLine
Range          = AddressAtom? '-' AddressAtom?
Brackets       = AddressAtom | '(' Address ')'
Negated        = ( Brackets | Range ) '!'?
Address        = ( Negated ',' )+ Negated

Substitute     = 's' Regex [^/]* '/' ( [1-9][0-9]* | 'g' )?
String         = '"' [^"]* '"' | "'" [^']* "'"
Quit           = 'q' [0-9]*
Keep           = 'k' ([1-9][0-9]*)? '-' ([1-9][0-9]*)?
Command        = [=pPlnthgxjJrzd] | '\' Character | Quit | Keep | String | Substitute

Instruction    = Address? Command*
Script         = ( Instruction ( ';' | '.' ) )* Instruction?
```

[`sed`]: https://www.gnu.org/software/sed/manual/sed.html
[Rust's Regex]: https://docs.rs/regex/latest/regex/
[verbose mode]: https://docs.rs/regex/latest/regex/?search=verbose#example-verbose-mode
[std::char::escape_default]: https://doc.rust-lang.org/std/primitive.char.html#method.escape_default
[similar way as `sed`]: https://www.gnu.org/software/sed/manual/sed.html#Execution-Cycle
