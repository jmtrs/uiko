#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use reqwest::{Client, redirect::Policy};
use serde::Deserialize;
use serde_json::{Value, json};
use uiko_capabilities::ParameterLocation;
use uiko_runtime_plan::{ReadOperationPlan, ReadRuntimePlan};
use url::Url;

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct GatewayState {
    plan: Arc<ReadRuntimePlan>,
    base_urls: Arc<BTreeMap<String, Url>>,
    client: Client,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryRequest {
    operation_id: String,
    #[serde(default)]
    input: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug)]
enum GatewayErrorKind {
    BadRequest,
    UnknownOperation,
    MissingConfiguration,
    UpstreamFailure,
    ResponseTooLarge,
}

#[derive(Debug)]
struct GatewayError {
    kind: GatewayErrorKind,
    message: String,
}

impl GatewayState {
    /// Build a read-only G0 gateway state from a compiled plan and deployment
    /// base URLs.
    ///
    /// # Errors
    ///
    /// Returns an error if the bounded HTTP client cannot be constructed.
    pub fn new(
        plan: ReadRuntimePlan,
        base_urls: BTreeMap<String, Url>,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self {
            plan: Arc::new(plan),
            base_urls: Arc::new(base_urls),
            client,
        })
    }
}

pub fn router(state: GatewayState) -> Router {
    Router::new()
        .route("/__uiko/query", post(invoke_query))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

async fn invoke_query(
    State(state): State<GatewayState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<Value>, GatewayError> {
    let operation = state
        .plan
        .operations
        .get(&request.operation_id)
        .ok_or_else(|| GatewayError {
            kind: GatewayErrorKind::UnknownOperation,
            message: format!("unknown logical query `{}`", request.operation_id),
        })?;

    let base_url = state
        .base_urls
        .get(&operation.provider_id)
        .ok_or_else(|| GatewayError {
            kind: GatewayErrorKind::MissingConfiguration,
            message: format!(
                "no deployment base URL configured for provider `{}`",
                operation.provider_id
            ),
        })?;

    let url = build_upstream_url(base_url, operation, &request.input)?;
    let response = state
        .client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| GatewayError {
            kind: GatewayErrorKind::UpstreamFailure,
            message: format!("upstream GET failed: {error}"),
        })?;

    if !response.status().is_success() {
        return Err(GatewayError {
            kind: GatewayErrorKind::UpstreamFailure,
            message: format!("upstream returned HTTP {}", response.status().as_u16()),
        });
    }

    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(GatewayError {
            kind: GatewayErrorKind::ResponseTooLarge,
            message: "upstream JSON response exceeds G0 size limit".into(),
        });
    }

    let bytes = response.bytes().await.map_err(|error| GatewayError {
        kind: GatewayErrorKind::UpstreamFailure,
        message: format!("cannot read upstream response: {error}"),
    })?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(GatewayError {
            kind: GatewayErrorKind::ResponseTooLarge,
            message: "upstream JSON response exceeds G0 size limit".into(),
        });
    }

    let value = serde_json::from_slice(&bytes).map_err(|error| GatewayError {
        kind: GatewayErrorKind::UpstreamFailure,
        message: format!("upstream response is not valid JSON: {error}"),
    })?;

    Ok(Json(value))
}

fn build_upstream_url(
    base_url: &Url,
    operation: &ReadOperationPlan,
    input: &BTreeMap<String, Value>,
) -> Result<Url, GatewayError> {
    let declared: BTreeSet<_> = operation
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();

    if let Some(name) = input.keys().find(|name| !declared.contains(name.as_str())) {
        return Err(GatewayError {
            kind: GatewayErrorKind::BadRequest,
            message: format!("undeclared query input `{name}`"),
        });
    }

    for parameter in operation
        .parameters
        .iter()
        .filter(|parameter| parameter.required)
    {
        if !input.contains_key(&parameter.name) {
            return Err(GatewayError {
                kind: GatewayErrorKind::BadRequest,
                message: format!("missing required query input `{}`", parameter.name),
            });
        }
    }

    let mut url = base_url.clone();
    {
        let mut segments = url.path_segments_mut().map_err(|()| GatewayError {
            kind: GatewayErrorKind::MissingConfiguration,
            message: "provider base URL cannot be used as an HTTP base URL".into(),
        })?;
        segments.pop_if_empty();

        for template_segment in operation
            .path
            .split('/')
            .filter(|segment| !segment.is_empty())
        {
            if let Some(name) = template_segment
                .strip_prefix('{')
                .and_then(|segment| segment.strip_suffix('}'))
            {
                let parameter = operation
                    .parameters
                    .iter()
                    .find(|parameter| {
                        parameter.name == name && parameter.location == ParameterLocation::Path
                    })
                    .ok_or_else(|| GatewayError {
                        kind: GatewayErrorKind::BadRequest,
                        message: format!("path template references undeclared input `{name}`"),
                    })?;
                let value = input.get(&parameter.name).ok_or_else(|| GatewayError {
                    kind: GatewayErrorKind::BadRequest,
                    message: format!("missing path input `{}`", parameter.name),
                })?;
                segments.push(&scalar_string(value, &parameter.name)?);
            } else if template_segment.contains('{') || template_segment.contains('}') {
                return Err(GatewayError {
                    kind: GatewayErrorKind::BadRequest,
                    message: "partial path parameter templates are not supported in G0".into(),
                });
            } else {
                segments.push(template_segment);
            }
        }
    }

    {
        let mut pairs = url.query_pairs_mut();
        for parameter in operation
            .parameters
            .iter()
            .filter(|parameter| parameter.location == ParameterLocation::Query)
        {
            if let Some(value) = input.get(&parameter.name) {
                pairs.append_pair(&parameter.name, &scalar_string(value, &parameter.name)?);
            }
        }
    }

    Ok(url)
}

fn scalar_string(value: &Value, name: &str) -> Result<String, GatewayError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => Err(GatewayError {
            kind: GatewayErrorKind::BadRequest,
            message: format!("query input `{name}` must be a JSON scalar"),
        }),
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = match self.kind {
            GatewayErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            GatewayErrorKind::UnknownOperation => StatusCode::NOT_FOUND,
            GatewayErrorKind::MissingConfiguration => StatusCode::SERVICE_UNAVAILABLE,
            GatewayErrorKind::UpstreamFailure | GatewayErrorKind::ResponseTooLarge => {
                StatusCode::BAD_GATEWAY
            }
        };
        let code = match self.kind {
            GatewayErrorKind::BadRequest => "UIKO_G0_BAD_INPUT",
            GatewayErrorKind::UnknownOperation => "UIKO_G0_UNKNOWN_QUERY",
            GatewayErrorKind::MissingConfiguration => "UIKO_G0_CONFIG",
            GatewayErrorKind::UpstreamFailure => "UIKO_G0_UPSTREAM",
            GatewayErrorKind::ResponseTooLarge => "UIKO_G0_RESPONSE_LIMIT",
        };

        (
            status,
            Json(json!({
                "status": "error",
                "code": code,
                "message": self.message,
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::{Json, Router, extract::Path, routing::get};
    use serde_json::json;
    use tokio::net::TcpListener;
    use uiko_capabilities::{OperationParameter, ParameterLocation};
    use uiko_runtime_plan::{ReadOperationPlan, ReadRuntimePlan};
    use url::Url;

    use super::{GatewayState, router};

    #[tokio::test]
    async fn logical_query_executes_one_real_upstream_get() {
        let upstream = Router::new().route(
            "/customers/{customer_id}",
            get(|Path(customer_id): Path<String>| async move {
                Json(json!({
                    "id": customer_id,
                    "name": "Alice"
                }))
            }),
        );
        let upstream_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("upstream listener");
        let upstream_addr = upstream_listener.local_addr().expect("upstream address");
        let upstream_task = tokio::spawn(async move {
            axum::serve(upstream_listener, upstream)
                .await
                .expect("upstream server");
        });

        let plan = ReadRuntimePlan {
            operations: BTreeMap::from([(
                "customers.CustomerDetail.query.customer".into(),
                ReadOperationPlan {
                    provider_id: "crm".into(),
                    path: "/customers/{customerId}".into(),
                    parameters: vec![OperationParameter {
                        name: "customerId".into(),
                        location: ParameterLocation::Path,
                        required: true,
                    }],
                },
            )]),
        };
        let state = GatewayState::new(
            plan,
            BTreeMap::from([(
                "crm".into(),
                Url::parse(&format!("http://{upstream_addr}/")).expect("base URL"),
            )]),
        )
        .expect("gateway state");

        let gateway_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("gateway listener");
        let gateway_addr = gateway_listener.local_addr().expect("gateway address");
        let gateway_task = tokio::spawn(async move {
            axum::serve(gateway_listener, router(state))
                .await
                .expect("gateway server");
        });

        let response = reqwest::Client::new()
            .post(format!("http://{gateway_addr}/__uiko/query"))
            .json(&json!({
                "operationId": "customers.CustomerDetail.query.customer",
                "input": { "customerId": "c 123" }
            }))
            .send()
            .await
            .expect("gateway request");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("JSON response");
        assert_eq!(body["id"], "c 123");
        assert_eq!(body["name"], "Alice");

        gateway_task.abort();
        upstream_task.abort();
    }

    #[tokio::test]
    async fn browser_cannot_choose_undeclared_transport_inputs() {
        let plan = ReadRuntimePlan {
            operations: BTreeMap::from([(
                "customers.CustomerDetail.query.customer".into(),
                ReadOperationPlan {
                    provider_id: "crm".into(),
                    path: "/customers/{customerId}".into(),
                    parameters: vec![OperationParameter {
                        name: "customerId".into(),
                        location: ParameterLocation::Path,
                        required: true,
                    }],
                },
            )]),
        };
        let state = GatewayState::new(
            plan,
            BTreeMap::from([(
                "crm".into(),
                Url::parse("http://127.0.0.1:9/").expect("base URL"),
            )]),
        )
        .expect("gateway state");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("gateway listener");
        let address = listener.local_addr().expect("gateway address");
        let task = tokio::spawn(async move {
            axum::serve(listener, router(state))
                .await
                .expect("gateway server");
        });

        let response = reqwest::Client::new()
            .post(format!("http://{address}/__uiko/query"))
            .json(&json!({
                "operationId": "customers.CustomerDetail.query.customer",
                "input": {
                    "customerId": "c-1",
                    "url": "https://example.invalid/"
                }
            }))
            .send()
            .await
            .expect("gateway request");

        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        task.abort();
    }
}
