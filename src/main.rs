use clap::Parser;
use se::{
    Editor, Error,
    Status::{self, *},
};
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

    #[command(flatten)]
    script: Script,

    /// Files that are processed
    #[arg(name = "FILE")]
    files: Vec<PathBuf>,
}

#[derive(Parser)]
#[group(multiple = true, required = true)]
struct Script {
    /// Commands that are executed
    #[arg(allow_hyphen_values = true)]
    command: Option<String>,

    /// Read the commands from the file
    #[arg(short = 'f', long = "file")]
    script: Option<PathBuf>,
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
    let args = parse_args();
    let editor = &mut unwrap!(get_editor(&args));
    let mut status = Normal;
    let mut count = 0;

    if args.files.is_empty() {
        let reader = BufReader::new(std::io::stdin());
        (status, count) = run(editor, reader, args.all);
    } else {
        for path in args.files.iter() {
            let file = unwrap!(File::open(path).map_err(Error::Io));
            let reader = BufReader::new(file);
            let (s, n) = run(editor, reader, args.all);
            count += n;
            if let Quit(_) = s {
                status = s;
                break;
            }
        }
    }

    if args.count {
        println!("{}", count)
    }
    if let Quit(code) = status {
        std::process::exit(code)
    }
}

fn parse_args() -> Args {
    let mut args = Args::parse();
    if args.script.script.is_some() {
        if let Some(arg) = args.script.command {
            args.files.insert(0, arg.into());
            args.script.command = None;
        }
    }
    args
}

fn get_editor(args: &Args) -> Result<Editor, Error> {
    if let Some(script) = &args.script.script {
        Editor::try_from(script.clone())
    } else if let Some(command) = &args.script.command {
        Editor::try_from(command.to_string())
    } else {
        unreachable!()
    }
}

fn run<R: BufRead>(editor: &mut Editor, reader: R, print_all: bool) -> (Status, usize) {
    let mut count = 0;
    let mut status = Normal;

    for line in reader.lines() {
        status = Normal;
        let mut buffer = unwrap!(line);

        if let Some((b, s)) = editor.process(&buffer) {
            buffer = b;
            status = s;
            count += 1;
        }

        if status == NoPrint {
            continue;
        }
        if print_all {
            println!("{}", buffer)
        }
        if let Quit(_) = status {
            break;
        }
    }

    (status, count)
}
