use crate::Line;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    // p
    Print,
    // l
    Escape,
    // =
    LineNumber,
    // n
    Newline,
    // "string" or 'string'
    Insert(String),
    // s/src/dst/[limit]
    Substitute(Replacer),
    // d
    Delete,
    // q[code]
    Quit(i32),
}

#[derive(Debug)]
pub(crate) struct Replacer {
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
    pub(crate) fn apply(&self, line: &mut Line) -> Action {
        use Command::*;
        match self {
            Print => println!("{}", line.1),
            Escape => println!("{}", line.1.escape_default()),
            LineNumber => print!("{:.10}", line.0),
            Newline => println!(),
            Insert(s) => print!("{}", s),
            Substitute(r) => line.1 = r.replace(&line.1),
            Delete => return Action::Skip,
            Quit(code) => return Action::Quit(*code),
        }
        Action::None
    }
}

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Skip,
    Quit(i32),
}
