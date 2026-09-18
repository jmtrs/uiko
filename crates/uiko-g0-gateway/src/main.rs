#![forbid(unsafe_code)]

use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf};

use tokio::net::TcpListener;
use uiko_compiler::compile;
use uiko_g0_gateway::{GatewayState, router};
use uiko_project::load_project;
use uiko_runtime_plan::derive_read_runtime_plan;
use url::Url;

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        eprintln!("{message}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let config = parse_args(&args)?;
    let loaded = load_project(&config.project).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let ir = compile(&loaded.source, &loaded.capabilities).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let plan = derive_read_runtime_plan(&ir, &loaded.openapi_transport)
        .map_err(|error| format!("UIKO_G0_PLAN: {error:?}"))?;
    let state = GatewayState::new(plan, config.integrations)
        .map_err(|error| format!("UIKO_G0_CLIENT: {error}"))?;

    let listener = TcpListener::bind(config.listen)
        .await
        .map_err(|error| format!("UIKO_G0_LISTEN: {error}"))?;
    println!("uiko G0 gateway listening on http://{}", config.listen);

    axum::serve(listener, router(state))
        .await
        .map_err(|error| format!("UIKO_G0_SERVE: {error}"))
}

struct Config {
    project: PathBuf,
    listen: SocketAddr,
    integrations: BTreeMap<String, Url>,
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut project = None;
    let mut listen = "127.0.0.1:3001"
        .parse::<SocketAddr>()
        .expect("static socket address");
    let mut integrations = BTreeMap::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--listen" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--listen requires HOST:PORT".to_string())?;
                listen = value
                    .parse()
                    .map_err(|error| format!("invalid --listen value `{value}`: {error}"))?;
                index += 2;
            }
            "--integration" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--integration requires ID=BASE_URL".to_string())?;
                let (id, raw_url) = value
                    .split_once('=')
                    .ok_or_else(|| "--integration requires ID=BASE_URL".to_string())?;
                if id.is_empty() {
                    return Err("integration id cannot be empty".into());
                }
                let url = Url::parse(raw_url)
                    .map_err(|error| format!("invalid base URL for `{id}`: {error}"))?;
                if !matches!(url.scheme(), "http" | "https") {
                    return Err(format!(
                        "integration `{id}` must use an http or https base URL"
                    ));
                }
                integrations.insert(id.to_string(), url);
                index += 2;
            }
            option if option.starts_with('-') => {
                return Err(format!("unknown option `{option}`"));
            }
            path if project.is_none() => {
                project = Some(PathBuf::from(path));
                index += 1;
            }
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }

    Ok(Config {
        project: project.unwrap_or_else(|| PathBuf::from(".")),
        listen,
        integrations,
    })
}
