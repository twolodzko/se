use crate::{Line, Regex};

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    /// p
    Println,
    /// P
    Print,
    /// l
    Escape,
    /// =
    LineNumber,
    /// "string" or 'string'
    Insert(String),
    /// s/src/dst/[limit]
    Substitute(Regex, String, usize),
    /// ks-e
    Keep(usize, Option<usize>),
    /// h
    Copy,
    /// g
    Paste,
    /// x
    Exchange,
    /// z
    Reset,
    /// d
    Delete,
    /// .
    Stop,
    /// q[code]
    Quit(i32),
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Normal,
    Next,
    NoPrint,
    Quit(i32),
}

impl From<&Command> for Status {
    fn from(value: &Command) -> Self {
        match value {
            Command::Delete => Status::NoPrint,
            Command::Stop => Status::Next,
            Command::Quit(code) => Status::Quit(*code),
            _ => Status::Normal,
        }
    }
}

impl Command {
    /// Run the command by modifying one of the three buffers: `pattern`, `hold`, or `print`
    /// and returning a status code.
    pub(crate) fn run(&self, pattern: &mut Line, hold: &mut String, print: &mut String) -> Status {
        use Command::*;
        match self {
            // commands that print things
            Println => {
                print.push_str(&pattern.1);
                print.push('\n');
            }
            Print => print.push_str(&pattern.1),
            Escape => {
                let escaped = pattern.1.escape_default().to_string();
                print.push_str(&escaped)
            }
            LineNumber => print.push_str(&pattern.0.to_string()),
            Insert(message) => print.push_str(message),
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
            Copy => {
                *hold = pattern.1.to_string();
            }
            Paste => {
                pattern.1 = hold.to_string();
            }
            Exchange => {
                std::mem::swap(hold, &mut pattern.1);
            }
            // commands that return special status codes
            Delete | Stop | Quit(_) => return Status::from(self),
        }
        Status::Normal
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Command::*;
        match self {
            Println => write!(f, "p"),
            Print => write!(f, "P"),
            Escape => write!(f, "l"),
            LineNumber => write!(f, "="),
            Insert(s) => write!(f, "'{}'", s),
            Substitute(r, t, l) => write!(f, "s/{}/{}/{}", r, t, l),
            Keep(s, None) => write!(f, "k{}-", s + 1),
            Keep(s, Some(t)) => write!(f, "k{}-{}", s + 1, s + t),
            Copy => write!(f, "h"),
            Paste => write!(f, "g"),
            Exchange => write!(f, "x"),
            Reset => write!(f, "z"),
            Delete => write!(f, "d"),
            Stop => write!(f, "."),
            Quit(c) => write!(f, "q{}", c),
        }
    }
}
