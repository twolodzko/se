use super::{reader::Reader, Error};
use anyhow::{bail, Result};

pub(crate) fn read_regex<R: Reader>(reader: &mut R) -> Result<String> {
    let mut acc = String::new();
    match reader.next()? {
        Some('/') => {
            read_until(reader, '/', false, &mut acc)?;
            acc.pop();
        }
        Some('^') => {
            acc.push('^');
            read_until(reader, '$', false, &mut acc)?;
        }
        Some(c) => bail!(Error::Unexpected(c)),
        _ => unreachable!(),
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
                        acc.push(c);
                    }
                    acc.push(e);
                } else {
                    acc.push(c);
                    bail!("escaped character is missing");
                }
            }
            '(' => {
                acc.push(c);
                verbose = read_brackets(reader, verbose, acc)?;
            }
            '#' if verbose => {
                acc.push(c);
                read_line(reader, acc)?;
            }
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
                '-' => {
                    if reader.next_is('x')? {
                        acc.push('x');
                        local_verbose = false;
                    }
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

fn read_line<R: Reader>(reader: &mut R, acc: &mut String) -> Result<()> {
    while let Some(c) = reader.next()? {
        acc.push(c);
        if c == '\n' {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::read_regex;
    use crate::parser::reader::StringReader;
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
        r"/(?x) # /comment/
        abc/<not this>",
        r"(?x) # /comment/
        abc";
        "verbose"
    )]
    #[test_case(
        r"/(?-x)#/<not this>",
        r"(?-x)#";
        "negated verbose"
    )]
    #[test_case(
        r"/(?x: # /comment/
        abc)#def/<not this>",
        r"(?x: # /comment/
        abc)#def";
        "inline verbose"
    )]
    #[test_case(
        r"/((?x) # /comment/
        abc)#def/<not this>",
        r"((?x) # /comment/
        abc)#def";
        "local verbose"
    )]
    #[test_case(
        r"/(?x) abc ((?-x) #/# ) # /comment//
        end/<not this>",
        r"(?x) abc ((?-x) #/# ) # /comment//
        end";
        "verbose canceled"
    )]
    #[test_case(
        r"^/$",
        r"^/$";
        "slash in whole line"
    )]
    #[test_case(
        r"^\/$",
        r"^/$";
        "escaped slash in whole line"
    )]
    #[test_case(
        r"^\\/$",
        r"^\\/$";
        "backslashes and unescaped slash in whole line"
    )]
    #[test_case(
        r"^\\\/$",
        r"^\\/$";
        "backslashes and escaped slash in whole line"
    )]
    fn read(input: &str, expected: &str) {
        let reader = &mut StringReader::from(input);
        let result = read_regex(reader).unwrap();
        assert_eq!(result, expected);
        regex::Regex::new(&result).expect("regex should parse");
    }
}
