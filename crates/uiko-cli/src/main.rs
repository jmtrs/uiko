#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        None | Some("--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            println!("uiko {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!(
                "UIKO0001: command `{command}` is not implemented in the bootstrap milestone"
            );
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "uiko {}\n\nBootstrap CLI. Semantic commands arrive with the compiler slice.\n\nUSAGE:\n    uiko [--help|--version]",
        env!("CARGO_PKG_VERSION")
    );
}
