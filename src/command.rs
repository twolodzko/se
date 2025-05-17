use crate::{Error, Line};

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
    /// {regex}
    Extract(Extract),
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

#[derive(Debug, Clone)]
pub struct Extract(regex::Regex, Option<String>);

impl Extract {
    pub(crate) fn new(s: &str) -> Result<Extract, Error> {
        let regex = regex::Regex::new(s).map_err(Error::Regex)?;
        let mut caps = regex.capture_names();
        caps.next();
        let key = match caps.next() {
            Some(Some(v)) => Some(v.to_string()),
            _ => None,
        };
        Ok(Extract(regex, key))
    }

    fn extract(&self, s: &str) -> String {
        if let Some(c) = self.0.captures(s) {
            if let Some(k) = &self.1 {
                let v = c.name(k).map_or(String::new(), |m| {
                    m.as_str().chars().fold(String::new(), |mut acc, c| {
                        if c == '"' {
                            acc.push('\\');
                        }
                        acc.push(c);
                        acc
                    })
                });
                return format!("{}=\"{}\"", k, v);
            }
            return if c.len() > 1 { c.get(1) } else { c.get(0) }
                .map_or("", |m| m.as_str())
                .to_string();
        }
        String::new()
    }
}

impl PartialEq for Extract {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
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
            Extract(e) => print!("{}", e.extract(&line.1)),
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

impl std::fmt::Display for Replacer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s/{}/{}/{}", self.regex, self.template, self.limit)
    }
}
