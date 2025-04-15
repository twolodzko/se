use crate::{Line, Regex};

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
    /// z
    Reset,
    /// d
    Delete,
    /// .
    Break,
    /// :label
    Label(String),
    /// b label
    GoTo(String, usize),
    /// q[code]
    Quit(i32),
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
    pub(crate) fn run<R: Iterator<Item = std::io::Result<Line>>>(
        &self,
        pattern: &mut Line,
        hold: &mut String,
        reader: &mut R,
    ) -> std::io::Result<Status> {
        use Command::*;
        match self {
            // commands that print things
            Println => println!("{}", pattern.1),
            Print => print!("{}", pattern.1),
            Escapeln => {
                let escaped = pattern.1.escape_default().to_string();
                println!("{}", escaped)
            }
            LineNumber => print!("{}", pattern.0),
            Insert(message) => print!("{}", message),
            // commands that modify the buffers
            Substitute(regex, template, limit) => {
                let replaced = regex.0.replacen(&pattern.1, *limit, template);
                pattern.1 = replaced.to_string()
            }
            Keep(skip, take) => {
                pattern.1 = if let Some(take) = take {
                    pattern.1.chars().skip(*skip).take(*take).collect()
                } else {
                    pattern.1.chars().skip(*skip).collect()
                };
            }
            Reset => pattern.1.clear(),
            Hold => {
                *hold = pattern.1.to_string();
            }
            Get => {
                pattern.1 = hold.to_string();
            }
            Exchange => {
                std::mem::swap(hold, &mut pattern.1);
            }
            Joinln => {
                pattern.1.push('\n');
                pattern.1.push_str(hold);
            }
            Join => {
                pattern.1.push_str(hold);
            }
            Readln(n) => {
                for _ in 0..*n {
                    if let Some(line) = reader.next() {
                        pattern.1.push('\n');
                        pattern.1.push_str(&line?.1);
                    } else {
                        break;
                    }
                }
            }
            // commands that return special status codes
            Delete => {
                pattern.1.clear();
                return Ok(Status::NoPrint);
            }
            Break | Quit(_) => return Ok(Status::from(self)),
            Label(_) => (),
            GoTo(_, _) => unreachable!(),
        }
        Ok(Status::Normal)
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Command::*;
        match self {
            Println => write!(f, "p"),
            Print => write!(f, "P"),
            Escapeln => write!(f, "l"),
            LineNumber => write!(f, "="),
            Insert(s) => write!(f, "'{}'", s),
            Substitute(r, t, l) => write!(f, "s/{}/{}/{}", r, t, l),
            Keep(s, None) => write!(f, "k{}-", s + 1),
            Keep(s, Some(t)) => write!(f, "k {}-{}", s + 1, s + t),
            Hold => write!(f, "h"),
            Get => write!(f, "g"),
            Exchange => write!(f, "x"),
            Joinln => write!(f, "j"),
            Join => write!(f, "J"),
            Readln(n) => write!(f, "r {}", n),
            Reset => write!(f, "z"),
            Delete => write!(f, "d"),
            Break => write!(f, "."),
            Quit(c) => write!(f, "q{}", c),
            Label(l) => write!(f, ":{}", l),
            GoTo(l, _) => write!(f, "b {}", l),
        }
    }
}
