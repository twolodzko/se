use super::reader::Reader;
use crate::{Error, Result, error};

pub(crate) fn read<R: Reader>(reader: &mut R) -> Result<(String, char)> {
    let mut acc = String::new();
    let delim;
    if let Some(c) = reader.peek()? {
        match c {
            '^' => {
                delim = '$';
                read_until(reader, '$', false, true, &mut acc)?;
            }
            _ => {
                delim = c;
                reader.skip();
                read_until(reader, c, false, false, &mut acc)?;
                acc.pop();
            }
        }
    } else {
        return Err(Error::EndOfInput);
    }
    Ok((acc, delim))
}

fn read_until<R: Reader>(
    reader: &mut R,
    delim: char,
    mut verbose: bool,
    whole_line: bool,
    acc: &mut String,
) -> Result<()> {
    while let Some(c) = reader.next()? {
        match c {
            '\\' if whole_line => {
                if let Some(e) = reader.next()? {
                    acc.push('\\');
                    acc.push(e);
                } else {
                    return error!("escaped character is missing");
                };
            }
            '\\' if let Some('\\') = reader.peek()? => {
                reader.skip();
                acc.push('\\');
            }
            '\\' if delim != '\\' => {
                let Some(e) = reader.next()? else {
                    return error!("escaped character is missing");
                };
                if e != delim {
                    acc.push('\\');
                }
                acc.push(e);
            }
            c if c == delim => {
                acc.push(c);
                return Ok(());
            }
            '(' => {
                acc.push(c);
                verbose = read_brackets(reader, verbose, whole_line, acc)?;
            }
            '#' if verbose => loop {
                if let Some('\n') = reader.next()? {
                    acc.push('\n');
                    break;
                }
            },
            _ => acc.push(c),
        }
    }
    Err(Error::Missing(delim))
}

fn read_brackets<R: Reader>(
    reader: &mut R,
    verbose: bool,
    whole_line: bool,
    acc: &mut String,
) -> Result<bool> {
    let mut local_verbose = verbose;
    if reader.next_is('?')? {
        acc.push('?');
        while let Some(c) = reader.next()? {
            acc.push(c);
            match c {
                // flag for inline definition
                ':' => {
                    read_until(reader, ')', local_verbose, whole_line, acc)?;
                    return Ok(verbose);
                }
                // finished reading the flag definition
                ')' => return Ok(local_verbose),
                // verbose flag
                'x' => local_verbose = true,
                '-' if reader.next_is('x')? => {
                    acc.push('x');
                    local_verbose = false;
                }

                // other flags
                _ => (),
            }
        }
        Err(Error::Missing(')'))
    } else {
        read_until(reader, ')', verbose, whole_line, acc)?;
        Ok(verbose)
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::StringReader;
    use test_case::test_case;

    #[test_case(
        r"//<not this>",
        r"";
        "empty"
    )]
    #[test_case(
        r"/abc/<not this>",
        r"abc";
        "constant"
    )]
    #[test_case(
        r"/\//<not this>",
        r"/";
        "slash"
    )]
    #[test_case(
        r"/\n\t/<not this>",
        r"\n\t";
        "escaped chars"
    )]
    #[test_case(
        r"^$<not this>",
        r"^$";
        "empty whole line"
    )]
    #[test_case(
        r"/(abc)/<not this>",
        r"(abc)";
        "brackets"
    )]
    #[test_case(
        r"/(a((b)(c)d)e(f))/<not this>",
        r"(a((b)(c)d)e(f))";
        "many brackets"
    )]
    #[test_case(
        r"/(?x)# /comment/
        abc/<not this>",
        r"(?x)
        abc";
        "verbose"
    )]
    #[test_case(
        r"/(?-x)#/<not this>",
        r"(?-x)#";
        "negated verbose"
    )]
    #[test_case(
        r"/(?x:# /comment/
        abc)#def/<not this>",
        r"(?x:
        abc)#def";
        "inline verbose"
    )]
    #[test_case(
        r"/((?x)# /comment/
        abc)#def/<not this>",
        r"((?x)
        abc)#def";
        "local verbose"
    )]
    #[test_case(
        r"/(?x) abc ((?-x) #/# )# /comment//
        end/<not this>",
        r"(?x) abc ((?-x) #/# )
        end";
        "verbose canceled"
    )]
    #[test_case(
        r"^/$",
        r"^/$";
        "slash in whole line"
    )]
    #[test_case(
        r"^\\/$",
        r"^\\/$";
        "backslashes and slash in whole line"
    )]
    #[test_case(
        r"^\\$",
        r"^\\$";
        "backslash in whole line"
    )]
    #[test_case(
        r"^\$$",
        r"^\$$";
        "only dollar in whole line"
    )]
    #[test_case(
        r"^/$",
        r"^/$";
        "only slash in whole line"
    )]
    #[test_case(
        r"/\//",
        r"/";
        "only slash"
    )]
    #[test_case(
        r"$\$$",
        r"$";
        "dollar with dollar delim"
    )]
    #[test_case(
        r"$\\\$$",
        r"\$";
        "dollar and slash with dollar delim"
    )]
    #[test_case(
        r".\..",
        r".";
        "only dot"
    )]
    #[test_case(
        r"#.#",
        r".";
        "only dot with hash delim"
    )]
    #[test_case(
        r"$\\\\$",
        r"\\";
        "only slash with dollar delim"
    )]
    #[test_case(
        r"/$/",
        r"$";
        "only dollar with slash delim"
    )]
    #[test_case(
        r"/^$/",
        r"^$";
        "delimited whole line pattern"
    )]
    #[test_case(
        r"#$#",
        r"$";
        "only dollar with hash delim"
    )]
    #[test_case(
        r"#\$#",
        r"\$";
        "escaped dollar with hash delim"
    )]
    #[test_case(
        r"\\\n\",
        r"\n";
        "newline with backslash delim"
    )]
    #[test_case(
        r"\foo\",
        r"foo";
        "backslash as delim"
    )]
    fn read(input: &str, expected: &str) {
        let reader = &mut StringReader::from(input);
        let (result, _) = super::read(reader).unwrap();
        assert_eq!(result, expected);
        regex::Regex::new(&result).expect("regex should parse");
    }

    #[test_case(r"/\\/"; "regular delim")]
    #[test_case(r"\\\\"; "custom delim")]
    fn read_single_backslash(input: &str) {
        // those are not valid regular expressions but worth to test as edge cases
        let reader = &mut StringReader::from(input);
        let (result, _) = super::read(reader).unwrap();
        assert_eq!(result, r"\");
    }

    #[test]
    fn errors_on_missing_trailing_backslash() {
        let reader = &mut StringReader::from(r"/abc\");
        let err = super::read(reader).unwrap_err();
        assert_eq!(err.to_string(), "escaped character is missing");
    }

    #[test]
    fn error_on_trailing_backslash() {
        let reader = &mut StringReader::from(r"^abc\");
        let err = super::read(reader).unwrap_err();
        assert_eq!(err.to_string(), "escaped character is missing");
    }
}
