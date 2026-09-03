use super::{Error, reader::Reader};
use anyhow::{Result, bail};

pub(crate) fn read<R: Reader>(reader: &mut R) -> Result<String> {
    let mut acc = String::new();
    if let Some(c) = reader.peek()? {
        match c {
            '/' => {
                reader.skip();
                read_until(reader, '/', false, &mut acc)?;
                acc.pop();
            }
            '^' => {
                read_until(reader, '$', false, &mut acc)?;
            }
            _ => bail!(Error::Unexpected(c)),
        }
    } else {
        bail!(Error::EndOfInput)
    }
    Ok(acc)
}

fn read_until<R: Reader>(
    reader: &mut R,
    delim: char,
    mut verbose: bool,
    acc: &mut String,
) -> Result<()> {
    while let Some(c) = reader.next()? {
        match c {
            c if c == delim => {
                acc.push(c);
                return Ok(());
            }
            '\\' => {
                if let Some(e) = reader.next()? {
                    if e != '/' {
                        acc.push('\\');
                    }
                    acc.push(e);
                } else {
                    bail!("escaped character is missing");
                }
            }
            '(' => {
                acc.push(c);
                verbose = read_brackets(reader, verbose, acc)?;
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
    bail!(Error::Missing(delim))
}

fn read_brackets<R: Reader>(reader: &mut R, verbose: bool, acc: &mut String) -> Result<bool> {
    let mut local_verbose = verbose;
    if reader.next_is('?')? {
        acc.push('?');
        while let Some(c) = reader.next()? {
            acc.push(c);
            match c {
                // flag for inline definition
                ':' => {
                    read_until(reader, ')', local_verbose, acc)?;
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
        bail!(Error::Missing(')'))
    } else {
        read_until(reader, ')', verbose, acc)?;
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
        r"^\$$",
        r"^\$$";
        "only dollar in whole line"
    )]
    #[test_case(
        r"^/$",
        r"^/$";
        "only slash in whole line"
    )]
    fn read(input: &str, expected: &str) {
        let reader = &mut StringReader::from(input);
        let result = super::read(reader).unwrap();
        assert_eq!(result, expected);
        regex::Regex::new(&result).expect("regex should parse");
    }
}
