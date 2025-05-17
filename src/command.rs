use crate::{Line, Regex};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
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
    /// `regex`
    Extract(Regex),
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
    /// no-op
    Nothing,
}

impl Command {
    pub(crate) fn apply(&self, line: &mut Line) {
        use Command::*;
        match self {
            Println => println!("{}", line.1),
            Print => print!("{}", line.1),
            Escape => println!("{}", line.1.escape_default()),
            LineNumber => print!("{:.10}", line.0),
            Insert(s) => print!("{}", s),
            Substitute(regex, template, limit) => {
                line.1 = regex.0.replacen(&line.1, *limit, template).to_string()
            }
            Extract(e) => {
                if let Some(c) = e.0.captures(&line.1) {
                    if let Some(s) = if c.len() > 1 { c.get(1) } else { c.get(0) } {
                        print!("{}", s.as_str());
                    }
                }
            }
            Reset => line.1.clear(),
            _ => (),
        }
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
            Extract(r) => write!(f, "{}", r.0),
            Copy => write!(f, "h"),
            Paste => write!(f, "g"),
            Exchange => write!(f, "x"),
            Reset => write!(f, "z"),
            Delete => write!(f, "d"),
            Stop => write!(f, "."),
            Quit(c) => write!(f, "q{}", c),
            Nothing => write!(f, ""),
        }
    }
}
