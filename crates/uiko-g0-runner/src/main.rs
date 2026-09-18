#![forbid(unsafe_code)]

use std::{
    fs,
    io::{self, Read as _},
    path::PathBuf,
    process::ExitCode,
};

use uiko_g0_runner::{aggregate_run, append_event};

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("G0 instrumentation error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(help());
    };

    match command {
        "append" => append(&args[1..]),
        "aggregate" => aggregate(&args[1..]),
        "--help" | "-h" => {
            println!("{}", help());
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{}", help())),
    }
}

fn append(args: &[String]) -> Result<(), String> {
    let trace = option_value(args, "--trace")?;
    let mut event_json = String::new();
    io::stdin()
        .read_to_string(&mut event_json)
        .map_err(|error| format!("cannot read event JSON from stdin: {error}"))?;
    append_event(PathBuf::from(trace).as_path(), event_json.trim())
}

fn aggregate(args: &[String]) -> Result<(), String> {
    let repo = option_value(args, "--repo")?;
    let trace = option_value(args, "--trace")?;
    let policy = option_value(args, "--path-policy")?;
    let output = option_value(args, "--output")?;

    let result = aggregate_run(
        PathBuf::from(repo).as_path(),
        PathBuf::from(trace).as_path(),
        PathBuf::from(policy).as_path(),
    )?;
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("cannot serialize run result: {error}"))?;

    if output == "-" {
        println!("{json}");
    } else {
        let output = PathBuf::from(output);
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create output directory: {error}"))?;
        }
        fs::write(&output, format!("{json}\n"))
            .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    }
    Ok(())
}

fn option_value<'a>(args: &'a [String], option: &str) -> Result<&'a str, String> {
    let position = args
        .iter()
        .position(|argument| argument == option)
        .ok_or_else(|| format!("missing required option {option}"))?;
    args.get(position + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn help() -> String {
    [
        "uiko-g0-runner",
        "",
        "USAGE:",
        "  uiko-g0-runner append --trace TRACE.ndjson < event.json",
        "  uiko-g0-runner aggregate --repo REPO --trace TRACE.ndjson \\",
        "    --path-policy experiments/g0/path-policy.json --output RESULT.json",
    ]
    .join("\n")
}
