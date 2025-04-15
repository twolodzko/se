use clap::Parser;
use seed::{parse, Action, Editor, Error};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(Parser)]
struct Args {
    /// Print all the lines (except the ones that were deleted)
    #[arg(short, long)]
    all: bool,

    /// Print the number of matches
    #[arg(short, long)]
    count: bool,

    /// Commands that are executed
    script: String,

    /// File that is processed
    file: Vec<PathBuf>,
}

macro_rules! unwrap {
    ( $f:expr ) => {
        $f.unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
            std::process::exit(1)
        })
    };
}

fn main() {
    use Action::*;

    let args = Args::parse();
    let editor = &mut unwrap!(parse(&args.script));

    let mut action = None;
    let mut count = 0;

    if args.file.is_empty() {
        let reader = BufReader::new(std::io::stdin());
        (action, count) = run(editor, reader, args.all);
    } else {
        for path in args.file.iter() {
            let file = unwrap!(File::open(path).map_err(Error::Io));
            let reader = BufReader::new(file);
            let (a, c) = run(editor, reader, args.all);
            count += c;
            if let Quit(_) = a {
                action = a;
                break;
            }
        }
    }

    if args.count {
        println!("{}", count)
    }
    if let Quit(code) = action {
        std::process::exit(code)
    }
}

fn run<R: BufRead>(editor: &mut Editor, reader: R, print_all: bool) -> (Action, usize) {
    use Action::*;

    let mut count = 0;
    let mut action = None;

    for line in reader.lines() {
        action = None;
        let mut buffer = unwrap!(line);

        if let Some((b, a)) = editor.apply(&buffer) {
            buffer = b;
            action = a;
            count += 1;
        }

        if action == Skip {
            continue;
        }
        if print_all {
            println!("{}", buffer)
        }
        if let Quit(_) = action {
            break;
        }
    }

    (action, count)
}
