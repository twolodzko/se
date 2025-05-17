use crate::Line;

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
    Substitute(Replacer),
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

#[derive(Debug, Clone)]
pub struct Replacer {
    pub(crate) regex: regex::Regex,
    pub(crate) template: String,
    pub(crate) limit: usize,
}

impl Replacer {
    fn replace(&self, input: &str) -> String {
        self.regex
            .replacen(input, self.limit, &self.template)
            .to_string()
    }
}

impl PartialEq for Replacer {
    fn eq(&self, other: &Self) -> bool {
        self.regex.as_str() == other.regex.as_str()
            && self.template == other.template
            && self.limit == other.limit
    }
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
            Substitute(r) => line.1 = r.replace(&line.1),
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
            Substitute(r) => write!(f, "{}", r),
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

impl std::fmt::Display for Replacer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s/{}/{}/{}", self.regex, self.template, self.limit)
    }
}
