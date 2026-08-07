use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};

use crate::error::report;
use crate::repl;
use crate::runtime;
use crate::vm::VM;

#[derive(ClapParser)]
#[command(name = "engling", about = "Engling — programming in plain English")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run an Engling source file
    Run {
        /// Engling source file to run
        file: PathBuf,
        /// Enable GUI mode (requires building with --features ui)
        #[arg(long)]
        ui: bool,
    },
}

pub fn run() {
    let args = Args::parse();

    match args.command {
        Some(Command::Run { file, ui }) => {
            let mut vm = VM::new();
            if let Err(e) = runtime::execute_file(&file, &mut vm) {
                report(&e);
                std::process::exit(1);
            }
            #[cfg(feature = "ui")]
            if ui {
                crate::ui::register_vm(&vm);
                if let Err(e) = crate::ui::run_event_loop() {
                    report(&e);
                    std::process::exit(1);
                }
            }
            #[cfg(not(feature = "ui"))]
            if ui {
                eprintln!("Warning: --ui flag passed but this build was compiled without `--features ui`.");
                eprintln!("Rebuild with `cargo build --features ui` to enable GUI support.");
            }
        }
        None => {
            repl::start();
        }
    }
}
