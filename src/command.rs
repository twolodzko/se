#[cfg(feature = "extract")]
use serde_json::{self, json};
#[cfg(feature = "extract")]
use std::collections::HashMap;

use crate::{Line, Regex};

#[allow(private_interfaces)]
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
    #[cfg(feature = "extract")]
    /// `regex`[n]
    Extract(Regex, usize),
    #[cfg(feature = "extract")]
    /// {regex}[n]
    JsonExtract(Regex, Vec<String>, usize),
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
            #[cfg(feature = "extract")]
            Extract(r, i) => {
                if let Some(c) = r.0.captures_iter(&line.1).nth(*i) {
                    if let Some(s) = if c.len() > 1 { c.get(1) } else { c.get(0) } {
                        print!("{}", s.as_str());
                    }
                }
            }
            #[cfg(feature = "extract")]
            JsonExtract(ref r, ref names, i) => {
                if *i > 0 {
                    if let Some(cap) = r.0.captures(&line.1) {
                        let s = json!(collect_captures(cap, names));
                        println!("{}", s)
                    }
                } else {
                    let s = json!(json_extract(r, names, &line.1));
                    println!("{}", s)
                };
            }
            Reset => line.1.clear(),
            _ => (),
        }
    }
}

#[cfg(feature = "extract")]
fn json_extract(regex: &Regex, names: &[String], haystack: &str) -> Vec<HashMap<String, String>> {
    regex
        .0
        .captures_iter(haystack)
        .fold(Vec::new(), |mut acc, cap| {
            let item = collect_captures(cap, names);
            acc.push(item);
            acc
        })
}

#[cfg(feature = "extract")]
fn collect_captures(cap: regex::Captures<'_>, names: &[String]) -> HashMap<String, String> {
    names.iter().fold(HashMap::new(), |mut acc, key| {
        if let Some(val) = cap.name(key) {
            acc.insert(key.to_string(), val.as_str().to_string());
        }
        acc
    })
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
            #[cfg(feature = "extract")]
            Extract(r, i) => write!(f, "`{}`{}", r.0, i),
            #[cfg(feature = "extract")]
            JsonExtract(r, _, i) => write!(f, "{{{}}}{}", r.0, i),
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
