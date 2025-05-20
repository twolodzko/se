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
    #[allow(private_interfaces)]
    Substitute(Regex, String, usize),
    /// y/src/dst/
    Translate(String, String),
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
            Translate(src, dst) => {
                line.1 = line.1.chars().map(|c| translate(c, src, dst)).collect()
            }
            Reset => line.1.clear(),
            _ => (),
        }
    }
}

fn translate(c: char, src: &str, dst: &str) -> char {
    for (s, d) in std::iter::zip(src.chars(), dst.chars().cycle()) {
        if c == s {
            return d;
        }
    }
    c
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
            Translate(s, d) => write!(f, "y/{}/{}/", s, d),
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
