use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const DEFAULT_API_URL: &str = "http://localhost:8080";

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    if let Err(error) = run_stdio(config).await {
        eprintln!("{}", error.message);
        std::process::exit(1);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    api_url: String,
    token: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            api_url: std::env::var("MEMORYNEXUS_API_URL")
                .unwrap_or_else(|_| DEFAULT_API_URL.to_string()),
            token: std::env::var("MEMORYNEXUS_TOKEN").ok(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiRequest {
    method: HttpMethod,
    url: String,
    body: Option<Value>,
    token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpError {
    message: String,
}

impl McpError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

async fn run_stdio(config: Config) -> Result<(), McpError> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|error| McpError::new(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Value =
            serde_json::from_str(&line).map_err(|error| McpError::new(error.to_string()))?;
        if let Some(response) = handle_jsonrpc_request(&config, &request).await {
            writeln!(stdout, "{response}").map_err(|error| McpError::new(error.to_string()))?;
            stdout
                .flush()
                .map_err(|error| McpError::new(error.to_string()))?;
        }
    }

    Ok(())
}

async fn handle_jsonrpc_request(config: &Config, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let id = id?;
    let result = match method {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(config, request.get("params").unwrap_or(&Value::Null)).await,
        _ => Err(McpError::new(format!("unknown MCP method: {method}"))),
    };

    Some(match result {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": error.message,
            },
        }),
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "memorynexus-mcp",
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            tool_schema(
                "list_spaces",
                "List Cognitive Spaces visible to the authenticated user.",
                json!({
                    "type": "object",
                    "properties": {},
                }),
            ),
            tool_schema(
                "add_memory",
                "Add a text memory to a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "content": {"type": "string"},
                        "title": {"type": "string"},
                        "tags": {"type": "array", "items": {"type": "string"}},
                        "memory_type": {"type": "string", "default": "text"},
                        "is_shared": {"type": "boolean", "default": false}
                    },
                    "required": ["content"]
                }),
            ),
            tool_schema(
                "search_memories",
                "Search memories by Cognitive Space or Lens.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "space_id": {"type": "string"},
                        "lens_id": {"type": "string"},
                        "semantic": {"type": "boolean", "default": false},
                        "limit": {"type": "integer", "default": 20}
                    },
                    "required": ["query"]
                }),
            ),
            tool_schema(
                "list_lenses",
                "List Lenses in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"}
                    },
                    "required": ["space_id"]
                }),
            ),
            tool_schema(
                "run_lens",
                "Run a Lens query and return a traceable Lens Run.",
                json!({
                    "type": "object",
                    "properties": {
                        "lens_id": {"type": "string"},
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "default": 5}
                    },
                    "required": ["lens_id", "query"]
                }),
            ),
            tool_schema(
                "get_lens_run",
                "Fetch a persisted Lens Run by ID.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": {"type": "string"}
                    },
                    "required": ["run_id"]
                }),
            ),
        ]
    })
}

fn tool_schema(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn build_api_request_for_tool(
    config: &Config,
    tool_name: &str,
    arguments: &Value,
) -> Result<ApiRequest, McpError> {
    let token = config
        .token
        .clone()
        .ok_or_else(|| McpError::new("MEMORYNEXUS_TOKEN is required"))?;
    let base_url = config.api_url.trim_end_matches('/');

    match tool_name {
        "list_spaces" => Ok(ApiRequest {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/spaces"),
            body: None,
            token,
        }),
        "add_memory" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/memories"),
            body: Some(json!({
                "space_id": optional_string(arguments, "space_id"),
                "content": required_string(arguments, "content")?,
                "title": optional_string(arguments, "title"),
                "tags": optional_string_array(arguments, "tags"),
                "memory_type": optional_string(arguments, "memory_type")
                    .unwrap_or_else(|| "text".to_string()),
                "is_shared": optional_bool(arguments, "is_shared").unwrap_or(false),
            })),
            token,
        }),
        "search_memories" => {
            let query = required_string(arguments, "query")?;
            let limit = optional_usize(arguments, "limit").unwrap_or(20).to_string();
            let semantic = optional_bool(arguments, "semantic").map(|value| value.to_string());
            let mut pairs = vec![("q", query.as_str()), ("limit", limit.as_str())];
            let space_id = optional_string(arguments, "space_id");
            let lens_id = optional_string(arguments, "lens_id");
            if let Some(semantic) = semantic.as_deref() {
                pairs.push(("semantic", semantic));
            }
            if let Some(space_id) = space_id.as_deref() {
                pairs.push(("space_id", space_id));
            }
            if let Some(lens_id) = lens_id.as_deref() {
                pairs.push(("lens_id", lens_id));
            }
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: with_query(&format!("{base_url}/api/v1/search"), &pairs)?,
                body: None,
                token,
            })
        }
        "list_lenses" => {
            let space_id = required_string(arguments, "space_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: with_query(
                    &format!("{base_url}/api/v1/lenses"),
                    &[("space_id", space_id.as_str())],
                )?,
                body: None,
                token,
            })
        }
        "run_lens" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/lens-runs"),
            body: Some(json!({
                "lens_id": required_string(arguments, "lens_id")?,
                "query": required_string(arguments, "query")?,
                "limit": optional_usize(arguments, "limit").unwrap_or(5),
            })),
            token,
        }),
        "get_lens_run" => {
            let run_id = required_string(arguments, "run_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: format!("{base_url}/api/v1/lens-runs/{run_id}"),
                body: None,
                token,
            })
        }
        _ => Err(McpError::new(format!("unknown tool: {tool_name}"))),
    }
}

async fn call_tool(config: &Config, params: &Value) -> Result<Value, McpError> {
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::new("tools/call params.name is required"))?;
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let request = build_api_request_for_tool(config, tool_name, arguments)?;
    let response = execute_api_request(request).await?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)
                    .map_err(|error| McpError::new(error.to_string()))?,
            }
        ],
        "isError": false,
    }))
}

async fn execute_api_request(request: ApiRequest) -> Result<Value, McpError> {
    let client = reqwest::Client::new();
    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
    }
    .bearer_auth(request.token);

    if let Some(body) = request.body {
        builder = builder.json(&body);
    }

    let response = builder
        .send()
        .await
        .map_err(|error| McpError::new(error.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| McpError::new(error.to_string()))?;
    let value = if text.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }))
    };

    if !status.is_success() {
        return Err(McpError::new(format!(
            "MemoryNexus API returned HTTP {}: {}",
            status.as_u16(),
            value
        )));
    }

    Ok(value)
}

fn required_string(arguments: &Value, key: &str) -> Result<String, McpError> {
    optional_string(arguments, key).ok_or_else(|| McpError::new(format!("{key} is required")))
}

fn optional_string(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn optional_bool(arguments: &Value, key: &str) -> Option<bool> {
    arguments.get(key).and_then(Value::as_bool)
}

fn optional_usize(arguments: &Value, key: &str) -> Option<usize> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn optional_string_array(arguments: &Value, key: &str) -> Vec<String> {
    arguments
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn with_query(base_url: &str, pairs: &[(&str, &str)]) -> Result<String, McpError> {
    let mut url =
        reqwest::Url::parse(base_url).map_err(|error| McpError::new(error.to_string()))?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_exposes_initial_mcp_surface() {
        let result = tools_list_result();
        let names: Vec<&str> = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();

        assert_eq!(
            names,
            vec![
                "list_spaces",
                "add_memory",
                "search_memories",
                "list_lenses",
                "run_lens",
                "get_lens_run",
            ]
        );
    }

    #[test]
    fn run_lens_tool_maps_to_lens_runs_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "run_lens",
            &json!({
                "lens_id": "lens-123",
                "query": "Summarize project direction",
                "limit": 3
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/lens-runs");
        assert_eq!(request.token, "jwt-token");
        assert_eq!(
            request.body,
            Some(json!({
                "lens_id": "lens-123",
                "query": "Summarize project direction",
                "limit": 3
            }))
        );
    }

    #[test]
    fn search_memories_tool_supports_lens_id() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "search_memories",
            &json!({
                "query": "cognitive lens",
                "lens_id": "lens-123",
                "limit": 5
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/search?q=cognitive+lens&limit=5&lens_id=lens-123"
        );
    }

    #[test]
    fn tool_calls_require_memorynexus_token() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };

        let error = build_api_request_for_tool(&config, "list_spaces", &json!({})).unwrap_err();

        assert_eq!(error.message, "MEMORYNEXUS_TOKEN is required");
    }
}
