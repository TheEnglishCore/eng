use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};

use crate::error::report;
use crate::package::{HttpFetcher, LocalFetcher, PackageStore};
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

    /// Install a package (by official name, GitHub URL, or .engpkg URL)
    Install {
        /// Package name or URL
        target: String,
    },

    /// Remove an installed package
    Remove {
        /// Package name
        name: String,
    },

    /// List installed packages
    List,

    /// Search the official package registry
    Search {
        /// Search query
        query: String,
    },

    /// Update installed packages that have a newer version available
    Update,
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
        Some(Command::Install { target }) => {
            if let Err(e) = run_install(&target) {
                report(&e);
                std::process::exit(1);
            }
        }
        Some(Command::Remove { name }) => {
            if let Err(e) = crate::package::remove_package(&name) {
                report(&e);
                std::process::exit(1);
            }
            println!("Removed package '{name}'.");
        }
        Some(Command::List) => match crate::package::list_installed() {
            Ok(packages) => print!("{}", crate::package::commands::format_list(&packages)),
            Err(e) => {
                report(&e);
                std::process::exit(1);
            }
        },
        Some(Command::Search { query }) => {
            let fetcher = build_fetcher();
            match crate::package::search_registry(&query, fetcher.as_ref()) {
                Ok(results) => print!("{}", crate::package::commands::format_search(&results)),
                Err(e) => {
                    report(&e);
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Update) => {
            let fetcher = build_fetcher();
            match crate::package::update_installed(fetcher.as_ref()) {
                Ok(updated) => {
                    if updated.is_empty() {
                        println!("All packages are already up to date.");
                    } else {
                        for line in updated {
                            println!("{line}");
                        }
                    }
                }
                Err(e) => {
                    report(&e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            repl::start();
        }
    }
}

/// Pick the best available fetcher for the CLI. Honours the
/// `ENGLING_LOCAL_REGISTRY` env var so a user can point at a directory
/// of mock registry + .engpkg files without touching the network.
fn build_fetcher() -> Box<dyn crate::package::Fetcher> {
    if let Ok(root) = std::env::var("ENGLING_LOCAL_REGISTRY") {
        if !root.trim().is_empty() {
            return Box::new(LocalFetcher::with_root(root));
        }
    }
    Box::new(HttpFetcher::new())
}

fn run_install(target: &str) -> Result<(), crate::error::EnglingError> {
    let fetcher = build_fetcher();
    let manifest = crate::package::install_source(target, fetcher.as_ref())?;
    println!(
        "Installed package '{}' ({}).",
        manifest.name, manifest.version
    );
    println!(
        "Location: {}",
        PackageStore::user_default()
            .package_dir(&manifest.name)
            .display()
    );
    Ok(())
}
