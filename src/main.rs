use clap::Parser;
use se::{
    parse,
    Command::{self, *},
    Editor, Error, FileReader, StringReader,
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
    let mut args = Args::parse();

    let res = if let Some(script) = args.script.script {
        if let Some(arg) = args.script.command {
            args.files.insert(0, arg.into());
            args.script.command = None;
        }
        parse(&mut unwrap!(FileReader::try_from(script)))
    } else {
        let command = args.script.command.unwrap();
        parse(&mut StringReader::from(command))
    };
    let editor = &mut unwrap!(res);

    let mut command = Nothing;
    let mut count = 0;

    if args.files.is_empty() {
        let reader = BufReader::new(std::io::stdin());
        (command, count) = run(editor, reader, args.all);
    } else {
        for path in args.files.iter() {
            let file = unwrap!(File::open(path).map_err(Error::Io));
            let reader = BufReader::new(file);
            let (c, n) = run(editor, reader, args.all);
            count += n;
            if let Quit(_) = c {
                command = c;
                break;
            }
        }
    }

    if args.count {
        println!("{}", count)
    }
    if let Command::Quit(code) = command {
        std::process::exit(code)
    }
}

fn run<R: BufRead>(editor: &mut Editor, reader: R, print_all: bool) -> (Command, usize) {
    let mut count = 0;
    let mut command = Nothing;

    for line in reader.lines() {
        command = Nothing;
        let mut buffer = unwrap!(line);

        if let Some((b, c)) = editor.apply(&buffer) {
            buffer = b;
            command = c;
            count += 1;
        }

        if command == Delete {
            continue;
        }
        if print_all {
            println!("{}", buffer)
        }
        if let Quit(_) = command {
            break;
        }
    }

    (command, count)
}
