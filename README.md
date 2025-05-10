# seed

## Syntax

`seed` is a command-line utility similar to [`sed`]. It can be called as

```
seed [OPTIONS] <SCRIPT> [FILE]...
```

where `<SCRIPT>` contains instructions (separated by `;`) of the form

```
[address][!][command]
```

The `command` is executed for each line from the input the `[FILE]`'s that matches the `address`.
While the syntax of the instruction is similar to `sed`'s, it is not the same and not meant to be so.
Rather than being a replacement, it is `sed`'s simplified cousin, using modernized syntax,
and written in Rust.

Same as `sed`, it can be used for string search and replace in files.

## How it works?

`seed` works in a [similar way as `sed`](https://www.gnu.org/software/sed/manual/sed.html#Execution-Cycle).
It uses a buffer, called *pattern space* in `sed`.

> `sed` operates by performing the following cycle on each line of input: first, `sed` reads one line from
> the input stream, removes any trailing newline, and places it in the pattern space.
> Then commands are executed; each command can have an address associated to it: addresses are a kind
> of condition code, and a command is only executed if the condition is verified before the command
> is to be executed.
>
> When the end of the script is reached, unless the `-n` option is in use, the contents of
> pattern space are printed out to the output stream, adding back the trailing newline if
> it was removed. Then the next cycle starts for the next input line.

The difference is that `seed` does not use the second buffer (*hold space*)
and by default works like `sed -n` (see below).

## Addresses

* Number like `1` or `278` points to a specific line. Line numbers start at 1.
* `1-5` an inclusive range of the lines between `1` and `5`.
  `-5` or `*-5` is equivalent to `1-5`.
  `1-` or `1-$` means all the lines from `1` to the final line.
* `*` or no address specified means that all the lines would match.
  If no address is given, this is the default.
* `$` never matches any line, so `5-$` (or `5-`) means a left-open interval.
  Using `$` anywhere but range end is pointless, as it is a no-op.
* `/regex/` matches the lines that match the regular expression specified between `/.../`.
  Regular expressions can be used as bounds of the ranges.
* `^regex$` can be used instead of `/^regex$/` when matching the whole line.
* `addr1,addr2,...,addrN` matches any of the addresses.
* `!` after the address negates it, e.g. `1!` means all the lines except the first.
* Addresses can be enclosed with brackets `(addr)`. It can be used together with negation,
  e.g. `(1,2,3)!` is equivalent to matching the `4-` range.

## Commands

* `p` - print the content of the buffer as-is.
* `l` - print the content of the buffer after escaping the characters with Rust's
  [std::char::escape_default](https://doc.rust-lang.org/std/primitive.char.html#method.escape_default).
* `s/src/dst/[limit]` - use regular expression to replace `src` with `dst` in the buffer.
* `=` - print the line number.
* `n` - print the newline character.
* `d` - clear the content of the buffer and immediately start processing next line.
* `"string"` or `'string'` - print the `string`. The `string` can contain special escape
  characters like `\n` or `\t`.
* `q [code]` - exit with the `code` exit code (0 by default).

## Multiple instructions

When script contains multiple instructions, they can be delimited with `;` or `.`.

* `;` is used for chaining instructions. After processing the instruction,
  the buffer would be processed using the following instruction.
* `.` marks the final instruction. If the address of the instruction would positively match,
  the processing of the line would stop after running the command,
  all the following instructions would be skipped.
  In a way, `.` works like the command `d`, but it does not clear the buffer.

For example, the script

```text
/sed/ ">> " p .
      "   " p
```

when applied to this README would print it's content prepending each line containing the word "sed"
with ">> " and every other line (no address) with "   ". If `;` was used instead of `.`, the
lines containing the word "sed" would be printed twice, because of matching addresses in the both instructions.

## Differences from `sed`

* Using [Rust's Regex] regular expression syntax, including the syntax for flags
  e.g. `/(?i)regex/` is used instead of `/regex/i`. The flags can be used in
  matches as well as substitutions.
* Using `$N` for substitutions instead of `\N`.
* Not using the command groups syntax `{ cmd1 ; cmd2 ; ... }`,
  but instead reading commands directly e.g. `=p` (actually `=np`, see above) is equivalent to `{ = ; p }` in `sed`.
* Only a subset of `sed` commands is supported and they can behave differently.
* Instead of `a string`, use `p"string"` to print the string after
  printing the line, same applies to `sed`s `i`.
* No multiline matches.
* No support for branching.
* `sed` by default prints all the lines unless explicitly deleted.
  To achieve this behavior use `-a` (`--all`) flag to print all the lines.
* In `sed` `$` means final line, here it means *never match*.
  As a consequence `5-$` would match all the lines starting from the fifth in both cases,
  but in `sed` the `$` would be the last line so the range would be finite,
  and here it would be infinite. Using `$` outside of range would never match.
* `seed` uses `s/src/dst/g` as a default rather than `s/src/dst/1` as `sed` does.


 [`sed`]: https://www.gnu.org/software/sed/manual/sed.html
 [Rust's Regex]: https://docs.rs/regex/latest/regex/
