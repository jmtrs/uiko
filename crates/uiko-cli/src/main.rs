#![forbid(unsafe_code)]

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use uiko_compiler::compile;
use uiko_core::{AppIr, Diagnostic, Severity};
use uiko_manifest::derive_ui_manifest;
use uiko_project::load_project;

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
        Some("validate") => {
            let validate_args: Vec<_> = args.collect();
            validate(&validate_args)
        }
        Some("build") => {
            let build_args: Vec<_> = args.collect();
            build(&build_args)
        }
        Some(command) => {
            eprintln!("UIKO0001: unknown command `{command}`");
            ExitCode::from(2)
        }
    }
}

fn validate(args: &[String]) -> ExitCode {
    let mut project = PathBuf::from(".");
    let mut project_set = false;
    let mut json = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("UIKO0002: --format requires a value");
                    return ExitCode::from(2);
                };
                if value != "json" {
                    eprintln!("UIKO0002: unsupported format `{value}`; expected `json`");
                    return ExitCode::from(2);
                }
                json = true;
                index += 2;
            }
            "--format=json" => {
                json = true;
                index += 1;
            }
            option if option.starts_with('-') => {
                eprintln!("UIKO0002: unknown validate option `{option}`");
                return ExitCode::from(2);
            }
            path if !project_set => {
                project = PathBuf::from(path);
                project_set = true;
                index += 1;
            }
            extra => {
                eprintln!("UIKO0002: unexpected validate argument `{extra}`");
                return ExitCode::from(2);
            }
        }
    }

    let ir = match compile_project(&project) {
        Ok(ir) => ir,
        Err(diagnostics) => return emit_diagnostics(&diagnostics, json),
    };

    if json {
        println!(
            "{{\"status\":\"ok\",\"app\":\"{}\",\"modules\":{}}}",
            json_escape(&ir.app_name),
            ir.modules.len()
        );
    } else {
        println!(
            "valid: {} ({} module{})",
            ir.app_name,
            ir.modules.len(),
            if ir.modules.len() == 1 { "" } else { "s" }
        );
    }

    ExitCode::SUCCESS
}

fn build(args: &[String]) -> ExitCode {
    let mut project = PathBuf::from(".");
    let mut project_set = false;
    let mut output: Option<PathBuf> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--ui-manifest" => {
                let Some(path) = args.get(index + 1) else {
                    eprintln!("UIKO0002: --ui-manifest requires an output path");
                    return ExitCode::from(2);
                };
                output = Some(PathBuf::from(path));
                index += 2;
            }
            option if option.starts_with('-') => {
                eprintln!("UIKO0002: unknown build option `{option}`");
                return ExitCode::from(2);
            }
            path if !project_set => {
                project = PathBuf::from(path);
                project_set = true;
                index += 1;
            }
            extra => {
                eprintln!("UIKO0002: unexpected build argument `{extra}`");
                return ExitCode::from(2);
            }
        }
    }

    let Some(output) = output else {
        eprintln!("UIKO0002: build requires --ui-manifest <OUTPUT>");
        return ExitCode::from(2);
    };

    let ir = match compile_project(&project) {
        Ok(ir) => ir,
        Err(diagnostics) => return emit_diagnostics(&diagnostics, false),
    };
    let manifest = derive_ui_manifest(&ir);

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
        && let Err(error) = fs::create_dir_all(parent)
    {
        eprintln!("UIKO0003: cannot create output directory: {error}");
        return ExitCode::from(2);
    }

    if let Err(error) = fs::write(&output, manifest.to_json_pretty()) {
        eprintln!(
            "UIKO0003: cannot write UI manifest `{}`: {error}",
            output.display()
        );
        return ExitCode::from(2);
    }

    println!("wrote UI manifest: {}", output.display());
    ExitCode::SUCCESS
}

fn compile_project(project: &Path) -> Result<AppIr, Vec<Diagnostic>> {
    let loaded = load_project(project)?;
    compile(&loaded.source, &loaded.capabilities)
}

fn emit_diagnostics(diagnostics: &[Diagnostic], json: bool) -> ExitCode {
    if json {
        println!("{}", diagnostics_json(diagnostics));
    } else {
        for diagnostic in diagnostics {
            eprintln!(
                "{} {}:{}-{} {}",
                diagnostic.code,
                diagnostic.span.source.as_str(),
                diagnostic.span.start,
                diagnostic.span.end,
                diagnostic.message
            );
        }
    }

    ExitCode::from(1)
}

fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::from("{\"status\":\"error\",\"diagnostics\":[");

    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            output,
            "{{\"code\":\"{}\",\"severity\":\"{}\",\"message\":\"{}\",\"source\":\"{}\",\"start\":{},\"end\":{}}}",
            json_escape(diagnostic.code),
            severity,
            json_escape(&diagnostic.message),
            json_escape(diagnostic.span.source.as_str()),
            diagnostic.span.start,
            diagnostic.span.end
        )
        .expect("writing JSON to String cannot fail");
    }

    output.push_str("]}");
    output
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control <= '\u{001f}' => {
                write!(escaped, "\\u{:04x}", u32::from(control))
                    .expect("writing JSON escape to String cannot fail");
            }
            other => escaped.push(other),
        }
    }

    escaped
}

fn print_help() {
    println!(
        "uiko {}\n\nUSAGE:\n    uiko [--help|--version]\n    uiko validate [PROJECT] [--format json]\n    uiko build [PROJECT] --ui-manifest <OUTPUT>\n\nCOMMANDS:\n    validate    load and compile a uiko project without browser execution\n    build       emit deterministic public build artifacts",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use uiko_core::{Diagnostic, SourceId, TextSpan};

    use super::{diagnostics_json, json_escape};

    #[test]
    fn json_escape_handles_machine_output_characters() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn diagnostics_json_contains_stable_machine_fields() {
        let diagnostics = vec![Diagnostic::error(
            "UIKO9999",
            "broken \"thing\"",
            TextSpan::new(SourceId::new("features/test.jsonc"), 4, 8),
        )];

        let json = diagnostics_json(&diagnostics);
        assert!(json.contains("\"code\":\"UIKO9999\""));
        assert!(json.contains("\"source\":\"features/test.jsonc\""));
        assert!(json.contains("\"start\":4"));
        assert!(json.contains("\"end\":8"));
    }
}
