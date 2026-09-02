use anyhow::Result;
use clap::Parser;
use se::{Program, Reader, Status};
use std::{io::Write, path::PathBuf, str::FromStr};

fn main() -> Result<()> {
    let args = parse_args();
    let mut program = Program::try_from(args.script)?;
    let mut reader = Reader::from(args.files);
    let out = &mut std::io::stdout().lock();

    let (status, count) = program.run(&mut reader, args.all, out)?;

    if args.count {
        writeln!(out, "{count}")?;
    }
    if let Status::Quit(code) = status {
        std::process::exit(code)
    }
    Ok(())
}

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
    path: Option<PathBuf>,
}

impl TryFrom<Script> for Program {
    type Error = anyhow::Error;
    fn try_from(value: Script) -> Result<Self, Self::Error> {
        if let Some(path) = &value.path {
            Program::try_from(path)
        } else if let Some(command) = &value.command {
            Program::from_str(command)
        } else {
            unreachable!()
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args::parse();
    if args.script.path.is_some()
        && let Some(arg) = args.script.command
    {
        // it's not a command, dumbo
        args.files.insert(0, arg.into());
        args.script.command = None;
    }
    args
}
