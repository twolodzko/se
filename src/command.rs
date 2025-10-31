use crate::{program::Memory, Line, Regex};
use anyhow::Result;
use std::io::{StdoutLock, Write};

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    /// p
    Println,
    /// P
    Print,
    /// l
    Escapeln,
    /// =
    LineNumber,
    /// "string" or 'string'
    Insert(String),
    /// s/src/dst/[limit]
    Substitute(Regex, String, usize),
    /// k s-e
    Keep(usize, Option<usize>),
    /// &
    GetLine,
    /// h
    Hold,
    /// g
    Get,
    /// x
    Exchange,
    /// j
    Joinln,
    /// J
    Join,
    /// r [num]
    Readln(usize),
    /// R
    ReadReplace,
    /// z
    Reset,
    /// d
    Delete,
    /// .
    Break,
    /// q [code]
    Quit(i32),
    /// e
    Eval,
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Normal,
    Break,
    NoPrint,
    Quit(i32),
}

impl From<&Command> for Status {
    fn from(value: &Command) -> Self {
        match value {
            Command::Delete => Status::NoPrint,
            Command::Break => Status::Break,
            Command::Quit(code) => Status::Quit(*code),
            _ => Status::Normal,
        }
    }
}

impl Command {
    /// Run the command by modifying one of the `pattern` or `hold` buffers
    /// and returning a status code.
    pub(crate) fn run<R: Iterator<Item = Result<Line>>>(
        &self,
        memory: &mut Memory,
        reader: &mut R,
        out: &mut StdoutLock,
    ) -> Result<Status> {
        use Command::*;
        match self {
            // commands that print things
            Println => writeln!(out, "{}", memory.this)?,
            Print => write!(out, "{}", memory.this)?,
            Escapeln => {
                let escaped = memory.this.escape_default().to_string();
                writeln!(out, "{escaped}")?
            }
            LineNumber => write!(out, "{}", memory.line.0)?,
            Insert(message) => write!(out, "{message}")?,
            // commands that modify the buffers
            Substitute(regex, template, limit) => {
                let replaced = regex.0.replacen(&memory.this, *limit, template);
                memory.this = replaced.to_string()
            }
            Keep(skip, take) => {
                memory.this = if let Some(take) = take {
                    memory.this.chars().skip(*skip).take(*take).collect()
                } else {
                    memory.this.chars().skip(*skip).collect()
                };
            }
            Reset => memory.this.clear(),
            Hold => {
                memory.hold = memory.this.to_string();
            }
            Get => {
                memory.this = memory.hold.to_string();
            }
            GetLine => memory.this = memory.line.1.to_string(),
            Exchange => {
                std::mem::swap(&mut memory.hold, &mut memory.this);
            }
            Joinln => {
                memory.this.push('\n');
                memory.this.push_str(&memory.hold);
            }
            Join => {
                memory.this.push_str(&memory.hold);
            }
            Readln(n) => {
                for _ in 0..*n {
                    if let Some(line) = reader.next() {
                        memory.this.push('\n');
                        memory.this.push_str(&line?.1);
                    } else {
                        break;
                    }
                }
            }
            ReadReplace => {
                if let Some(line) = reader.next() {
                    memory.read(line?);
                } else {
                    return Ok(Status::Break);
                }
            }
            // commands that return special status codes
            Delete => {
                memory.this.clear();
                return Ok(Status::NoPrint);
            }
            Break | Quit(_) => return Ok(Status::from(self)),
            Eval => {
                let (stdout, code) = eval_sh(&memory.this)?;
                memory.this = stdout;
                if let Some(code) = code {
                    return Ok(Status::Quit(code));
                }
            }
        }
        Ok(Status::Normal)
    }
}

fn eval_sh(cmd: &str) -> Result<(String, Option<i32>)> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()?;
    if !out.stderr.is_empty() {
        std::io::stderr().write_all(&out.stderr)?;
    }
    let stdout = std::str::from_utf8(&out.stdout)?.to_string();
    let code = match out.status.code() {
        Some(0) => None,
        Some(code) => Some(code),
        None => Some(0),
    };
    Ok((stdout, code))
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Command::*;
        match self {
            Println => write!(f, "p"),
            Print => write!(f, "P"),
            Escapeln => write!(f, "l"),
            LineNumber => write!(f, "="),
            Insert(s) => write!(f, "'{s}'"),
            Substitute(r, t, l) => write!(f, "s/{r}/{t}/{l}"),
            Keep(s, None) => write!(f, "k {}-", s + 1),
            Keep(s, Some(t)) => write!(f, "k {}-{}", s + 1, s + t),
            Hold => write!(f, "h"),
            Get => write!(f, "g"),
            GetLine => write!(f, "&"),
            Exchange => write!(f, "x"),
            Joinln => write!(f, "j"),
            Join => write!(f, "J"),
            Readln(n) => write!(f, "r {n}"),
            ReadReplace => write!(f, "R"),
            Reset => write!(f, "z"),
            Delete => write!(f, "d"),
            Break => write!(f, "."),
            Quit(c) => write!(f, "q {c}"),
            Eval => write!(f, "e"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::{lines::MockReader, program::Memory, Line};

    #[test]
    fn readln() {
        let example = [1, 2, 3, 4, 5];
        let mut reader = example.iter().map(|n| Ok(Line(*n, n.to_string())));
        let mut memory = Memory::default();
        memory.read(Line(0, "start".to_string()));

        Command::Readln(1)
            .run(&mut memory, &mut reader, &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(memory.this, "start\n1");

        Command::Readln(4)
            .run(&mut memory, &mut reader, &mut std::io::stdout().lock())
            .unwrap();
        assert_eq!(memory.this, "start\n1\n2\n3\n4\n5");
    }

    #[test]
    fn join() {
        let mut memory = Memory::default();
        memory.read(Line(0, "one".to_string()));
        memory.hold = "two".to_string();

        Command::Join
            .run(
                &mut memory,
                &mut MockReader {},
                &mut std::io::stdout().lock(),
            )
            .unwrap();
        assert_eq!(memory.this, "onetwo");
    }

    #[test]
    fn joinln() {
        let mut memory = Memory::default();
        memory.read(Line(0, "one".to_string()));
        memory.hold = "two".to_string();

        Command::Joinln
            .run(
                &mut memory,
                &mut MockReader {},
                &mut std::io::stdout().lock(),
            )
            .unwrap();
        assert_eq!(memory.this, "one\ntwo");
    }

    #[test]
    fn exchange() {
        let mut memory = Memory::default();
        memory.read(Line(0, "one".to_string()));
        memory.hold = "two".to_string();

        Command::Exchange
            .run(
                &mut memory,
                &mut MockReader {},
                &mut std::io::stdout().lock(),
            )
            .unwrap();
        assert_eq!(memory.this, "two");
        assert_eq!(memory.hold, "one");
    }
}
