use crate::{Reader, Regex, program::Memory};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::io::Write;
use unescaper::unescape;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Command {
    /// p[string]
    Println(Option<String>),
    /// P[string]
    Print(Option<String>),
    /// a"string"
    Append(String),
    /// i"string"
    Prepend(String),
    /// l
    Escape,
    /// L
    UnEscape,
    /// u
    ToUrl,
    /// U
    FromUrl,
    /// b
    ToBase64,
    /// B
    FromBase64,
    /// =
    LineNumber,
    /// s/src/dst/[limit]
    Substitute(Regex, String, usize),
    /// k s-e
    Keep(usize, Option<usize>),
    /// &
    CancelEdits,
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
    /// k
    Collectln,
    /// K
    Collect,
    /// r [num]
    Readln(usize),
    /// R
    ReadReplace,
    /// z
    Reset(Option<String>),
    /// d
    Delete,
    /// .
    Break,
    /// q [code]
    Quit(u8),
    /// e
    Eval,
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Normal,
    Break,
    NoPrint,
    Quit(u8),
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
    pub(crate) fn run(
        &self,
        memory: &mut Memory,
        reader: &mut Reader,
        out: &mut dyn Write,
    ) -> Result<Status> {
        use Command::*;
        match self {
            // commands that print things
            Println(None) => writeln!(out, "{}", memory.this)?,
            Println(Some(s)) => writeln!(out, "{}", s)?,
            Print(None) => write!(out, "{}", memory.this)?,
            Print(Some(s)) => write!(out, "{}", s)?,
            LineNumber => write!(out, "{}", memory.line.0)?,
            // edit
            Append(s) => memory.this.push_str(s),
            Prepend(s) => memory.this = s.to_owned() + &memory.this,
            Escape => {
                memory.this = memory.this.escape_default().to_string();
            }
            UnEscape => {
                memory.this = unescape(&memory.this)?;
            }
            ToUrl => {
                memory.this = urlencoding::encode(&memory.this).into_owned();
            }
            FromUrl => {
                memory.this = urlencoding::decode(&memory.this)?.into_owned();
            }
            ToBase64 => {
                memory.this = BASE64_STANDARD.encode(&memory.this);
            }
            FromBase64 => {
                let b = BASE64_STANDARD.decode(&memory.this)?;
                memory.this = String::from_utf8_lossy(&b).into_owned();
            }
            // commands that modify the buffers
            Substitute(regex, template, limit) => {
                let replaced = regex.0.replacen(&memory.this, *limit, template);
                memory.this = replaced.into_owned()
            }
            Keep(skip, take) => {
                memory.this = if let Some(take) = take {
                    memory.this.chars().skip(*skip).take(*take).collect()
                } else {
                    memory.this.chars().skip(*skip).collect()
                };
            }
            Reset(None) => memory.this.clear(),
            Reset(Some(s)) => memory.this = s.to_owned(),
            Hold => {
                memory.hold = memory.this.clone();
            }
            Get => {
                memory.this = memory.hold.clone();
            }
            CancelEdits => memory.this = memory.line.1.clone(),
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
            Collectln => {
                memory.hold.push('\n');
                memory.hold.push_str(&memory.this);
            }
            Collect => {
                memory.hold.push_str(&memory.this);
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

fn eval_sh(cmd: &str) -> Result<(String, Option<u8>)> {
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
        Some(code) => Some(u8::try_from(code).unwrap_or(255)),
        None => Some(0),
    };
    Ok((stdout, code))
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Command::*;
        match self {
            Println(None) => write!(f, "p"),
            Println(Some(s)) => write!(f, "p'{}'", s),
            Print(None) => write!(f, "P"),
            Print(Some(s)) => write!(f, "P'{}'", s),
            Append(s) => write!(f, "a'{}'", s),
            Prepend(s) => write!(f, "i'{}'", s),
            Escape => write!(f, "l"),
            UnEscape => write!(f, "L"),
            ToUrl => write!(f, "u"),
            FromUrl => write!(f, "U"),
            ToBase64 => write!(f, "b"),
            FromBase64 => write!(f, "B"),
            LineNumber => write!(f, "="),
            Substitute(r, t, l) => write!(f, "s/{}/{}/{}", r, t, l),
            Keep(s, None) => write!(f, "c{}-", s + 1),
            Keep(s, Some(t)) => write!(f, "c{}-{}", s + 1, s + t),
            Hold => write!(f, "h"),
            Get => write!(f, "g"),
            CancelEdits => write!(f, "o"),
            Exchange => write!(f, "x"),
            Joinln => write!(f, "j"),
            Join => write!(f, "J"),
            Collectln => write!(f, "k"),
            Collect => write!(f, "K"),
            Readln(n) => write!(f, "r{}", n),
            ReadReplace => write!(f, "R"),
            Reset(None) => write!(f, "z"),
            Reset(Some(s)) => write!(f, "z'{}'", s),
            Delete => write!(f, "d"),
            Break => write!(f, "."),
            Quit(c) => write!(f, "q{}", c),
            Eval => write!(f, "e"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::{Line, Reader, program::Memory};

    #[test]
    fn readln() {
        let iter = (1..=5).into_iter().map(|n| Ok(n.to_string()));
        let mut reader = Reader::new(iter);
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
                &mut Reader::empty(),
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
                &mut Reader::empty(),
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
                &mut Reader::empty(),
                &mut std::io::stdout().lock(),
            )
            .unwrap();
        assert_eq!(memory.this, "two");
        assert_eq!(memory.hold, "one");
    }
}
