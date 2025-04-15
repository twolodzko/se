use crate::{Error, Memory, Reader, Regex, Result, error};
use std::{borrow::Cow, io::Write};
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
    /// =
    LineNumber,
    /// s/src/dst/[limit]
    Substitute(Regex, String, usize),
    /// k s-e
    Keep(usize, Option<usize>),
    /// o
    CancelEdits,
    /// h [string]
    Hold(Option<String>),
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
    /// r[num]
    Readln(usize),
    /// R
    ReadReplace,
    /// z
    Reset(Option<String>),
    /// d
    Delete,
    /// .
    Break,
    /// q[code]
    Quit(u8),
    /// e[command]
    Eval(Option<String>),
    /// :label
    Label(String),
    /// b label
    Branch(String, usize),
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Normal,
    Break,
    NoPrint,
    GoTo(usize),
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
            Prepend(s) => memory.this.insert_str(0, s),
            Escape => {
                memory.this = memory.this.escape_default().to_string();
            }
            UnEscape => {
                memory.this = unescape(&memory.this)?;
            }
            // commands that modify the buffers
            Substitute(regex, template, limit) => {
                if let Cow::Owned(replaced) = regex.0.replacen(&memory.this, *limit, template) {
                    memory.this = replaced;
                }
            }
            Keep(skip, take) => {
                memory.this = if let Some(take) = take {
                    memory.this.chars().skip(*skip).take(*take).collect()
                } else {
                    memory.this.chars().skip(*skip).collect()
                };
            }
            Reset(o) => {
                memory.this.clear();
                if let Some(s) = o {
                    memory.this.push_str(s);
                }
            }
            Hold(o) => {
                memory.hold.clear();
                if let Some(s) = o {
                    memory.hold.push_str(s);
                } else {
                    memory.hold.push_str(&memory.this);
                }
            }
            Get => {
                memory.this.clear();
                memory.this.push_str(&memory.hold);
            }
            CancelEdits => {
                memory.this.clear();
                memory.this.push_str(&memory.line.1);
            }
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
            // branching
            Label(_) => {}
            Branch(_, pos) => return Ok(Status::GoTo(*pos)),
            // commands that return special status codes
            Delete => {
                memory.this.clear();
                return Ok(Status::NoPrint);
            }
            Break | Quit(_) => return Ok(Status::from(self)),
            Eval(cmd) => {
                let cmd = match cmd {
                    Some(s) => s,
                    None => &memory.this,
                };
                let out = &eval_sh(cmd)?;
                memory.this.clear();
                memory.this.push_str(out);
            }
        }
        Ok(Status::Normal)
    }
}

fn eval_sh(cmd: &str) -> Result<String> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        if out.stderr.is_empty() {
            error!("evaluating '{}' failed", cmd)
        } else {
            let s = String::from_utf8_lossy(&out.stderr);
            error!("{}", s.strip_suffix('\n').unwrap_or(&s))
        }
    }
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
            LineNumber => write!(f, "="),
            Substitute(r, t, l) => write!(f, "s/{}/{}/{}", r, t, l),
            Keep(s, None) => write!(f, "c{}-", s + 1),
            Keep(s, Some(t)) => write!(f, "c{}-{}", s + 1, s + t),
            Hold(None) => write!(f, "h"),
            Hold(Some(s)) => write!(f, "h'{}'", s),
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
            Eval(None) => write!(f, "e"),
            Eval(Some(s)) => write!(f, "e'{}'", s),
            Label(s) => write!(f, ":{}", s),
            Branch(s, _) => write!(f, "b {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use crate::{Line, Memory, Reader};

    #[test]
    fn readln() {
        let iter = (b'1'..=b'5').into_iter().map(|n| Ok(vec![n]));
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
