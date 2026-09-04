use crate::{Reader, Regex, program::Memory};
use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use core::str;
use std::io::Write;
use unescaper::unescape;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Command {
    /// p
    Println,
    /// P
    Print,
    /// l
    Escape,
    /// L
    UnEscape,
    /// t
    ToHtml,
    /// T
    FromHtml,
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
    /// "string" or 'string'
    Insert(String),
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
            Println => writeln!(out, "{}", memory.this)?,
            Print => write!(out, "{}", memory.this)?,
            Escape => {
                memory.this = memory.this.escape_default().to_string();
            }
            UnEscape => {
                memory.this = unescape(&memory.this)?;
            }
            ToHtml => {
                memory.this = html_escape(&memory.this);
            }
            FromHtml => {
                memory.this = html_unescape(&memory.this);
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
            LineNumber => write!(out, "{}", memory.line.0)?,
            Insert(message) => write!(out, "{message}")?,
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
            Reset => memory.this.clear(),
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

fn html_escape(s: &str) -> String {
    let mut acc = String::new();
    for c in s.chars() {
        let s = match c {
            '"' => "&quot;",
            '\'' => "&#39;",
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '/' => "&#x2F;",
            _ => {
                acc.push(c);
                continue;
            }
        };
        acc.push_str(s)
    }
    acc
}

fn html_unescape(s: &str) -> String {
    let mut acc = String::new();
    let mut iter = s.chars();
    while let Some(c) = iter.next() {
        if c == '&' {
            let mut s = String::from(c);
            for c in iter.by_ref() {
                s.push(c);
                if c == ';' {
                    break;
                }
            }
            let c = match s.as_str() {
                "&quot;" => '"',
                "&#39;" => '\'',
                "&amp;" => '&',
                "&lt;" => '<',
                "&gt;" => '>',
                "&#x2F;" => '/',
                _ => {
                    acc.push_str(&s);
                    continue;
                }
            };
            acc.push(c);
        } else {
            acc.push(c);
        }
    }
    acc
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Command::*;
        match self {
            Println => write!(f, "p"),
            Print => write!(f, "P"),
            Escape => write!(f, "l"),
            UnEscape => write!(f, "L"),
            ToHtml => write!(f, "t"),
            FromHtml => write!(f, "T"),
            ToUrl => write!(f, "u"),
            FromUrl => write!(f, "U"),
            ToBase64 => write!(f, "b"),
            FromBase64 => write!(f, "B"),
            LineNumber => write!(f, "="),
            Insert(s) => write!(f, "'{s}'"),
            Substitute(r, t, l) => write!(f, "s/{}/{}/{}", r, t, l),
            Keep(s, None) => write!(f, "k {}-", s + 1),
            Keep(s, Some(t)) => write!(f, "k {}-{}", s + 1, s + t),
            Hold => write!(f, "h"),
            Get => write!(f, "g"),
            CancelEdits => write!(f, "c"),
            Exchange => write!(f, "x"),
            Joinln => write!(f, "j"),
            Join => write!(f, "J"),
            Readln(n) => write!(f, "r{}", n),
            ReadReplace => write!(f, "R"),
            Reset => write!(f, "z"),
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
