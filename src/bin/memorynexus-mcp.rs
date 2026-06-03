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
    Patch,
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
                "create_space",
                "Create a Cognitive Space for personal, family, project, or organization memory.",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "description": {"type": "string"},
                        "space_type": {"type": "string", "enum": ["personal", "family", "project", "organization"], "default": "personal"}
                    },
                    "required": ["name"]
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
                "create_lens",
                "Create a Lens interpretation strategy in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "name": {"type": "string"},
                        "description": {"type": "string"},
                        "strategy": {"type": "string", "default": "default"},
                        "output_format": {"type": "string", "default": "summary"},
                        "retrieval_mode": {"type": "string", "default": "semantic"}
                    },
                    "required": ["space_id", "name"]
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
            tool_schema(
                "get_profile",
                "Project and persist a compact Cognitive Profile for a personal agent.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "lens_id": {"type": "string"},
                        "target": {"type": "string", "default": "personal_context"},
                        "limit": {"type": "integer", "default": 12}
                    }
                }),
            ),
            tool_schema(
                "add_reminder",
                "Create a scheduled recall reminder in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "content": {"type": "string"},
                        "remind_at": {"type": "string", "description": "RFC3339 timestamp, for example 2026-05-26T09:00:00Z"},
                        "title": {"type": "string"},
                        "memory_id": {"type": "string"},
                        "repeat_rule": {"type": "string", "enum": ["daily", "weekly", "monthly"]},
                        "delivery_channel": {"type": "string", "enum": ["in_app"], "default": "in_app"}
                    },
                    "required": ["space_id", "content", "remind_at"]
                }),
            ),
            tool_schema(
                "list_reminders",
                "List scheduled recall reminders in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "due_only": {"type": "boolean", "default": false},
                        "include_completed": {"type": "boolean", "default": false},
                        "limit": {"type": "integer", "default": 20}
                    },
                    "required": ["space_id"]
                }),
            ),
            tool_schema(
                "complete_reminder",
                "Mark a pending reminder as completed.",
                json!({
                    "type": "object",
                    "properties": {
                        "reminder_id": {"type": "string"}
                    },
                    "required": ["reminder_id"]
                }),
            ),
            tool_schema(
                "mark_reminder_delivery",
                "Record in-app reminder delivery as delivered or failed.",
                json!({
                    "type": "object",
                    "properties": {
                        "reminder_id": {"type": "string"},
                        "status": {"type": "string", "enum": ["delivered", "failed"]},
                        "error": {"type": "string"}
                    },
                    "required": ["reminder_id", "status"]
                }),
            ),
            tool_schema(
                "route_agent_context",
                "Recommend whether an agent should write, search, run a Lens, get a profile, or ignore context.",
                json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"},
                        "space_id": {"type": "string"},
                        "lens_id": {"type": "string"},
                        "target": {"type": "string", "default": "personal_context"}
                    },
                    "required": ["message"]
                }),
            ),
            tool_schema(
                "learning_math_create_practice_session",
                "Create a parent-assisted learning.math practice session in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "namespace_id": {"type": "string", "description": "Optional existing learning.math Namespace ID"},
                        "practice_goal": {"type": "string"},
                        "exercise": {"type": "string"},
                        "answer": {"type": "string"},
                        "reasoning": {"type": "string"},
                        "mistake_pattern": {"type": "string"},
                        "feedback": {"type": "string"},
                        "practice_adjustment": {"type": "string"},
                        "next_exercise": {"type": "string"},
                        "status": {"type": "string", "enum": ["active", "completed", "paused"]},
                        "capture_memory": {"type": "boolean", "default": false}
                    },
                    "required": ["space_id", "practice_goal", "exercise"]
                }),
            ),
            tool_schema(
                "learning_math_record_attempt",
                "Record the child's answer or reasoning for a learning.math practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "practice_session_id": {"type": "string"},
                        "answer": {"type": "string"},
                        "reasoning": {"type": "string"},
                        "capture_memory": {"type": "boolean", "default": false}
                    },
                    "required": ["practice_session_id"]
                }),
            ),
            tool_schema(
                "learning_math_record_feedback",
                "Record parent feedback, mistake pattern, adjustment, and next exercise for a learning.math practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "practice_session_id": {"type": "string"},
                        "mistake_pattern": {"type": "string"},
                        "feedback": {"type": "string"},
                        "practice_adjustment": {"type": "string"},
                        "next_exercise": {"type": "string"},
                        "status": {"type": "string", "enum": ["active", "completed", "paused"]},
                        "capture_memory": {"type": "boolean", "default": false}
                    },
                    "required": ["practice_session_id"]
                }),
            ),
            tool_schema(
                "learning_math_list_practice_sessions",
                "List learning.math practice sessions in a Cognitive Space.",
                json!({
                    "type": "object",
                    "properties": {
                        "space_id": {"type": "string"},
                        "namespace_id": {"type": "string"},
                        "limit": {"type": "integer", "default": 20},
                        "offset": {"type": "integer", "default": 0}
                    },
                    "required": ["space_id"]
                }),
            ),
            tool_schema(
                "learning_math_get_practice_session",
                "Fetch one learning.math practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "practice_session_id": {"type": "string"}
                    },
                    "required": ["practice_session_id"]
                }),
            ),
            tool_schema(
                "get_install_status",
                "Inspect the local MemoryNexus checkout, local MCP binary version, and API health before deciding whether to install or upgrade.",
                json!({
                    "type": "object",
                    "properties": {
                        "checkout_dir": {"type": "string"}
                    }
                }),
            ),
            tool_schema(
                "upgrade_install",
                "Plan or apply a local MemoryNexus upgrade. Defaults to plan-only; set apply=true to run local commands.",
                json!({
                    "type": "object",
                    "properties": {
                        "checkout_dir": {"type": "string"},
                        "apply": {"type": "boolean", "default": false},
                        "pull": {"type": "boolean", "default": false},
                        "rebuild_mcp": {"type": "boolean", "default": false},
                        "rebuild_api": {"type": "boolean", "default": false},
                        "skip_tests": {"type": "boolean", "default": false}
                    }
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
        "create_space" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/spaces"),
            body: Some(json!({
                "name": required_string(arguments, "name")?,
                "description": optional_string(arguments, "description"),
                "space_type": optional_string(arguments, "space_type")
                    .unwrap_or_else(|| "personal".to_string()),
            })),
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
        "create_lens" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/lenses"),
            body: Some(json!({
                "space_id": required_string(arguments, "space_id")?,
                "name": required_string(arguments, "name")?,
                "description": optional_string(arguments, "description"),
                "strategy": optional_string(arguments, "strategy")
                    .unwrap_or_else(|| "default".to_string()),
                "output_format": optional_string(arguments, "output_format")
                    .unwrap_or_else(|| "summary".to_string()),
                "retrieval_mode": optional_string(arguments, "retrieval_mode")
                    .unwrap_or_else(|| "semantic".to_string()),
            })),
            token,
        }),
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
        "get_profile" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/profiles"),
            body: Some(json!({
                "space_id": optional_string(arguments, "space_id"),
                "lens_id": optional_string(arguments, "lens_id"),
                "target": optional_string(arguments, "target")
                    .unwrap_or_else(|| "personal_context".to_string()),
                "limit": optional_usize(arguments, "limit").unwrap_or(12),
            })),
            token,
        }),
        "add_reminder" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/reminders"),
            body: Some(json!({
                "space_id": required_string(arguments, "space_id")?,
                "memory_id": optional_string(arguments, "memory_id"),
                "title": optional_string(arguments, "title"),
                "content": required_string(arguments, "content")?,
                "remind_at": required_string(arguments, "remind_at")?,
                "repeat_rule": optional_string(arguments, "repeat_rule"),
                "delivery_channel": optional_string(arguments, "delivery_channel"),
            })),
            token,
        }),
        "list_reminders" => {
            let space_id = required_string(arguments, "space_id")?;
            let due_only = optional_bool(arguments, "due_only")
                .unwrap_or(false)
                .to_string();
            let include_completed = optional_bool(arguments, "include_completed")
                .unwrap_or(false)
                .to_string();
            let limit = optional_usize(arguments, "limit").unwrap_or(20).to_string();

            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: with_query(
                    &format!("{base_url}/api/v1/reminders"),
                    &[
                        ("space_id", space_id.as_str()),
                        ("due_only", due_only.as_str()),
                        ("include_completed", include_completed.as_str()),
                        ("limit", limit.as_str()),
                    ],
                )?,
                body: None,
                token,
            })
        }
        "mark_reminder_delivery" => {
            let reminder_id = required_string(arguments, "reminder_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Post,
                url: format!("{base_url}/api/v1/reminders/{reminder_id}/delivery"),
                body: Some(json!({
                    "status": required_string(arguments, "status")?,
                    "error": optional_string(arguments, "error"),
                })),
                token,
            })
        }
        "complete_reminder" => {
            let reminder_id = required_string(arguments, "reminder_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Post,
                url: format!("{base_url}/api/v1/reminders/{reminder_id}/complete"),
                body: None,
                token,
            })
        }
        "route_agent_context" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/agent/route"),
            body: Some(json!({
                "message": required_string(arguments, "message")?,
                "space_id": optional_string(arguments, "space_id"),
                "lens_id": optional_string(arguments, "lens_id"),
                "target": optional_string(arguments, "target")
                    .unwrap_or_else(|| "personal_context".to_string()),
            })),
            token,
        }),
        "learning_math_create_practice_session" => Ok(ApiRequest {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/learning/math/practice-sessions"),
            body: Some(json!({
                "space_id": required_string(arguments, "space_id")?,
                "namespace_id": optional_string(arguments, "namespace_id"),
                "practice_goal": required_string(arguments, "practice_goal")?,
                "exercise": required_string(arguments, "exercise")?,
                "answer": combined_answer(arguments),
                "mistake_pattern": optional_string(arguments, "mistake_pattern"),
                "feedback": optional_string(arguments, "feedback"),
                "practice_adjustment": optional_string(arguments, "practice_adjustment"),
                "next_exercise": optional_string(arguments, "next_exercise"),
                "status": optional_string(arguments, "status"),
                "capture_memory": optional_bool(arguments, "capture_memory").unwrap_or(false),
            })),
            token,
        }),
        "learning_math_record_attempt" => {
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Patch,
                url: format!(
                    "{base_url}/api/v1/learning/math/practice-sessions/{session_id}/attempt"
                ),
                body: Some(json!({
                    "answer": combined_answer(arguments),
                    "capture_memory": optional_bool(arguments, "capture_memory").unwrap_or(false),
                })),
                token,
            })
        }
        "learning_math_record_feedback" => {
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Patch,
                url: format!(
                    "{base_url}/api/v1/learning/math/practice-sessions/{session_id}/feedback"
                ),
                body: Some(json!({
                    "mistake_pattern": optional_string(arguments, "mistake_pattern"),
                    "feedback": optional_string(arguments, "feedback"),
                    "practice_adjustment": optional_string(arguments, "practice_adjustment"),
                    "next_exercise": optional_string(arguments, "next_exercise"),
                    "status": optional_string(arguments, "status"),
                    "capture_memory": optional_bool(arguments, "capture_memory").unwrap_or(false),
                })),
                token,
            })
        }
        "learning_math_list_practice_sessions" => {
            let space_id = required_string(arguments, "space_id")?;
            let limit = optional_usize(arguments, "limit").unwrap_or(20).to_string();
            let offset = optional_usize(arguments, "offset").unwrap_or(0).to_string();
            let namespace_id = optional_string(arguments, "namespace_id");
            let mut pairs = vec![
                ("space_id", space_id.as_str()),
                ("limit", limit.as_str()),
                ("offset", offset.as_str()),
            ];
            if let Some(namespace_id) = namespace_id.as_deref() {
                pairs.insert(1, ("namespace_id", namespace_id));
            }
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: with_query(
                    &format!("{base_url}/api/v1/learning/math/practice-sessions"),
                    &pairs,
                )?,
                body: None,
                token,
            })
        }
        "learning_math_get_practice_session" => {
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: format!("{base_url}/api/v1/learning/math/practice-sessions/{session_id}"),
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
    if is_local_tool(tool_name) {
        let response = call_local_tool(config, tool_name, arguments).await?;
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&response)
                        .map_err(|error| McpError::new(error.to_string()))?,
                }
            ],
            "isError": false,
        }));
    }

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

fn is_local_tool(tool_name: &str) -> bool {
    matches!(tool_name, "get_install_status" | "upgrade_install")
}

async fn call_local_tool(
    config: &Config,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, McpError> {
    match tool_name {
        "get_install_status" => {
            let checkout =
                resolve_checkout_dir(optional_string(arguments, "checkout_dir").as_deref())?;
            let api_health = fetch_api_health(config).await.unwrap_or_else(|error| {
                json!({
                    "reachable": false,
                    "error": error.message,
                })
            });

            Ok(json!({
                "local": local_version_data(),
                "checkout": checkout_status(&checkout),
                "api": api_health,
                "upgrade": upgrade_plan_value(false, false, true, false),
            }))
        }
        "upgrade_install" => {
            let checkout =
                resolve_checkout_dir(optional_string(arguments, "checkout_dir").as_deref())?;
            let apply = optional_bool(arguments, "apply").unwrap_or(false);
            let pull = optional_bool(arguments, "pull").unwrap_or(false);
            let rebuild_mcp = optional_bool(arguments, "rebuild_mcp").unwrap_or(false);
            let rebuild_api = optional_bool(arguments, "rebuild_api").unwrap_or(false);
            let skip_tests = optional_bool(arguments, "skip_tests").unwrap_or(false);
            let plan = upgrade_plan_value(pull, rebuild_mcp, !skip_tests, rebuild_api);

            if !apply {
                return Ok(json!({
                    "mode": "plan",
                    "checkout": checkout.display().to_string(),
                    "plan": plan,
                    "apply_hint": "call upgrade_install with apply=true to execute these local commands",
                }));
            }

            let dirty = git_status_short(&checkout)?;
            if pull && !dirty.trim().is_empty() {
                return Err(McpError::new(
                    "refusing to git pull with local changes; commit/stash them or rerun with pull=false",
                ));
            }

            let mut results = Vec::new();
            if pull {
                results.push(run_local_command(&checkout, "git", &["pull"])?);
            }
            if !skip_tests {
                results.push(run_local_command(&checkout, "cargo", &["test"])?);
            }
            if rebuild_mcp {
                results.push(run_local_command(
                    &checkout,
                    "cargo",
                    &["build", "--bin", "memorynexus-mcp"],
                )?);
            }
            if rebuild_api {
                results.push(run_local_command(
                    &checkout,
                    "cargo",
                    &["build", "--bin", "memorynexus"],
                )?);
            }

            Ok(json!({
                "mode": "applied",
                "checkout": checkout.display().to_string(),
                "plan": plan,
                "results": results,
                "restart_required": {
                    "api": rebuild_api,
                    "mcp_client": true,
                    "note": "Restart the API after backend changes and reload the agent MCP client so it starts a fresh memorynexus-mcp process."
                }
            }))
        }
        _ => Err(McpError::new(format!("unknown local tool: {tool_name}"))),
    }
}

async fn fetch_api_health(config: &Config) -> Result<Value, McpError> {
    let base_url = config.api_url.trim_end_matches('/');
    let response = reqwest::Client::new()
        .get(format!("{base_url}/api/v1/health"))
        .send()
        .await
        .map_err(|error| McpError::new(error.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| McpError::new(error.to_string()))?;
    let body = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }));

    Ok(json!({
        "reachable": status.is_success(),
        "status_code": status.as_u16(),
        "body": body,
    }))
}

async fn execute_api_request(request: ApiRequest) -> Result<Value, McpError> {
    let client = reqwest::Client::new();
    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
        HttpMethod::Patch => client.patch(&request.url),
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

fn combined_answer(arguments: &Value) -> Option<String> {
    match (
        optional_string(arguments, "answer"),
        optional_string(arguments, "reasoning"),
    ) {
        (Some(answer), Some(reasoning)) => Some(format!("{answer}\n\nReasoning: {reasoning}")),
        (Some(answer), None) => Some(answer),
        (None, Some(reasoning)) => Some(format!("Reasoning: {reasoning}")),
        (None, None) => None,
    }
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

fn local_version_data() -> Value {
    json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "mcp_server": "memorynexus-mcp",
        "binary": std::env::current_exe()
            .ok()
            .map(|path| path.display().to_string()),
    })
}

fn resolve_checkout_dir(checkout_dir: Option<&str>) -> Result<std::path::PathBuf, McpError> {
    let path = checkout_dir
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .map_err(|error| McpError::new(error.to_string()))?;
    if !path.join("Cargo.toml").exists() {
        return Err(McpError::new(format!(
            "{} does not look like the MemoryNexus checkout",
            path.display()
        )));
    }
    Ok(path)
}

fn checkout_status(checkout: &std::path::Path) -> Value {
    let git_head = run_local_command(checkout, "git", &["log", "-1", "--oneline"]).ok();
    let git_status = git_status_short(checkout).ok();

    json!({
        "path": checkout.display().to_string(),
        "git_head": git_head.and_then(|result| result.get("stdout").cloned()),
        "dirty": git_status
            .as_deref()
            .map(|status| !status.trim().is_empty())
            .unwrap_or(false),
        "git_status_short": git_status,
    })
}

fn git_status_short(checkout: &std::path::Path) -> Result<String, McpError> {
    let output = std::process::Command::new("git")
        .arg("status")
        .arg("--short")
        .current_dir(checkout)
        .output()
        .map_err(|error| McpError::new(error.to_string()))?;
    if !output.status.success() {
        return Err(McpError::new(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_local_command(
    checkout: &std::path::Path,
    program: &str,
    args: &[&str],
) -> Result<Value, McpError> {
    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(checkout)
        .output()
        .map_err(|error| McpError::new(error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(McpError::new(format!(
            "{} {} failed: {}",
            program,
            args.join(" "),
            stderr
        )));
    }

    Ok(json!({
        "command": std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" "),
        "stdout": stdout,
        "stderr": stderr,
    }))
}

fn upgrade_plan_value(pull: bool, rebuild_mcp: bool, run_tests: bool, rebuild_api: bool) -> Value {
    let mut steps = Vec::new();
    steps.push(json!({
        "command": "git status --short",
        "reason": "detect local edits before any source update",
    }));
    if pull {
        steps.push(json!({
            "command": "git pull",
            "reason": "update source from the configured remote; skipped when local edits are already the desired upgrade",
        }));
    }
    if run_tests {
        steps.push(json!({
            "command": "cargo test",
            "reason": "verify the updated checkout before reconnecting agents",
        }));
    }
    if rebuild_mcp {
        steps.push(json!({
            "command": "cargo build --bin memorynexus-mcp",
            "reason": "refresh built-binary MCP installs",
        }));
    }
    if rebuild_api {
        steps.push(json!({
            "command": "cargo build --bin memorynexus",
            "reason": "refresh built-binary API installs",
        }));
    }
    steps.push(json!({
        "command": "restart API and reload MCP client",
        "reason": "running processes keep old code until restarted",
    }));

    json!({
        "steps": steps,
        "notes": [
            "Skip git pull when the checkout already contains the user's local edits.",
            "Skip cargo build --bin memorynexus-mcp when the MCP config uses cargo run.",
            "Restart the API after backend code or migrations change; migrations run on API startup."
        ]
    })
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
                "create_space",
                "add_memory",
                "search_memories",
                "list_lenses",
                "create_lens",
                "run_lens",
                "get_lens_run",
                "get_profile",
                "add_reminder",
                "list_reminders",
                "complete_reminder",
                "mark_reminder_delivery",
                "route_agent_context",
                "learning_math_create_practice_session",
                "learning_math_record_attempt",
                "learning_math_record_feedback",
                "learning_math_list_practice_sessions",
                "learning_math_get_practice_session",
                "get_install_status",
                "upgrade_install",
            ]
        );
    }

    #[test]
    fn upgrade_plan_defaults_to_safe_plan_only_steps() {
        let plan = upgrade_plan_value(false, false, true, false);
        let steps = plan["steps"].as_array().unwrap();

        assert_eq!(steps[0]["command"], "git status --short");
        assert!(steps.iter().any(|step| step["command"] == "cargo test"));
        assert!(steps
            .iter()
            .any(|step| step["command"] == "restart API and reload MCP client"));
    }

    #[tokio::test]
    async fn upgrade_install_tool_is_local_and_does_not_require_token_for_plan() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };

        let result = call_local_tool(
            &config,
            "upgrade_install",
            &json!({
                "apply": false,
                "pull": true,
                "rebuild_mcp": true
            }),
        )
        .await
        .unwrap();

        assert_eq!(result["mode"], "plan");
        assert_eq!(result["plan"]["steps"][1]["command"], "git pull");
        assert!(result["apply_hint"]
            .as_str()
            .unwrap()
            .contains("apply=true"));
    }

    #[test]
    fn create_space_tool_maps_to_spaces_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "create_space",
            &json!({
                "name": "Personal Agent Space",
                "description": "Agent memory universe",
                "space_type": "personal"
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/spaces");
        assert_eq!(
            request.body,
            Some(json!({
                "name": "Personal Agent Space",
                "description": "Agent memory universe",
                "space_type": "personal"
            }))
        );
    }

    #[test]
    fn create_lens_tool_maps_to_lenses_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "create_lens",
            &json!({
                "space_id": "space-123",
                "name": "Personal Context",
                "strategy": "personal_context",
                "output_format": "brief",
                "retrieval_mode": "semantic"
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/lenses");
        assert_eq!(
            request.body,
            Some(json!({
                "space_id": "space-123",
                "name": "Personal Context",
                "description": null,
                "strategy": "personal_context",
                "output_format": "brief",
                "retrieval_mode": "semantic"
            }))
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

    #[test]
    fn get_profile_tool_maps_to_profiles_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "get_profile",
            &json!({
                "space_id": "space-123",
                "target": "personal_context",
                "limit": 8
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/profiles");
        assert_eq!(
            request.body,
            Some(json!({
                "space_id": "space-123",
                "lens_id": null,
                "target": "personal_context",
                "limit": 8,
            }))
        );
    }

    #[test]
    fn get_profile_tool_allows_default_space() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "get_profile",
            &json!({
                "target": "personal_context",
                "limit": 8
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/profiles");
        assert_eq!(
            request.body,
            Some(json!({
                "space_id": null,
                "lens_id": null,
                "target": "personal_context",
                "limit": 8,
            }))
        );
    }

    #[test]
    fn route_agent_context_tool_maps_to_agent_route_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "route_agent_context",
            &json!({
                "message": "Remember this: I prefer Rust.",
                "space_id": "space-123"
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/agent/route");
        assert_eq!(
            request.body,
            Some(json!({
                "message": "Remember this: I prefer Rust.",
                "space_id": "space-123",
                "lens_id": null,
                "target": "personal_context",
            }))
        );
    }

    #[test]
    fn reminder_tools_map_to_reminders_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let add = build_api_request_for_tool(
            &config,
            "add_reminder",
            &json!({
                "space_id": "space-123",
                "title": "Review",
                "content": "Review Rust practice",
                "remind_at": "2026-05-26T09:00:00Z",
                "repeat_rule": "weekly",
                "delivery_channel": "in_app"
            }),
        )
        .unwrap();
        let list = build_api_request_for_tool(
            &config,
            "list_reminders",
            &json!({
                "space_id": "space-123",
                "due_only": true,
                "limit": 5
            }),
        )
        .unwrap();
        let complete = build_api_request_for_tool(
            &config,
            "complete_reminder",
            &json!({
                "reminder_id": "reminder-123"
            }),
        )
        .unwrap();
        let failed_delivery = build_api_request_for_tool(
            &config,
            "mark_reminder_delivery",
            &json!({
                "reminder_id": "reminder-123",
                "status": "failed",
                "error": "client notification panel unavailable"
            }),
        )
        .unwrap();

        assert_eq!(add.method, HttpMethod::Post);
        assert_eq!(add.url, "http://localhost:8080/api/v1/reminders");
        assert_eq!(
            add.body,
            Some(json!({
                "space_id": "space-123",
                "memory_id": null,
                "title": "Review",
                "content": "Review Rust practice",
                "remind_at": "2026-05-26T09:00:00Z",
                "repeat_rule": "weekly",
                "delivery_channel": "in_app",
            }))
        );
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/reminders?space_id=space-123&due_only=true&include_completed=false&limit=5"
        );
        assert_eq!(complete.method, HttpMethod::Post);
        assert_eq!(
            complete.url,
            "http://localhost:8080/api/v1/reminders/reminder-123/complete"
        );
        assert_eq!(failed_delivery.method, HttpMethod::Post);
        assert_eq!(
            failed_delivery.url,
            "http://localhost:8080/api/v1/reminders/reminder-123/delivery"
        );
        assert_eq!(
            failed_delivery.body,
            Some(json!({
                "status": "failed",
                "error": "client notification panel unavailable",
            }))
        );
    }

    #[test]
    fn tools_list_exposes_learning_math_practice_tools() {
        let result = tools_list_result();
        let names: Vec<&str> = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();

        for name in [
            "learning_math_create_practice_session",
            "learning_math_record_attempt",
            "learning_math_record_feedback",
            "learning_math_list_practice_sessions",
            "learning_math_get_practice_session",
        ] {
            assert!(names.contains(&name), "{name} should be registered");
        }
    }

    #[test]
    fn learning_math_tools_require_memorynexus_token() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };

        let error = build_api_request_for_tool(
            &config,
            "learning_math_create_practice_session",
            &json!({
                "space_id": "space-123",
                "practice_goal": "Improve fraction word problems",
                "exercise": "Solve five fraction word problems"
            }),
        )
        .unwrap_err();

        assert_eq!(error.message, "MEMORYNEXUS_TOKEN is required");
    }

    #[test]
    fn learning_math_create_practice_session_maps_to_practice_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "learning_math_create_practice_session",
            &json!({
                "space_id": "space-123",
                "namespace_id": "namespace-123",
                "practice_goal": "Improve fraction word problems",
                "exercise": "Solve five fraction word problems",
                "answer": "I solved 3 of 5",
                "reasoning": "I added the numerators first",
                "mistake_pattern": "Mixed units",
                "feedback": "Label units before calculating",
                "practice_adjustment": "Add a unit-labeling step",
                "next_exercise": "Try three unit-conversion problems",
                "capture_memory": true
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/learning/math/practice-sessions"
        );
        assert_eq!(request.token, "jwt-token");
        assert_eq!(
            request.body,
            Some(json!({
                "space_id": "space-123",
                "namespace_id": "namespace-123",
                "practice_goal": "Improve fraction word problems",
                "exercise": "Solve five fraction word problems",
                "answer": "I solved 3 of 5\n\nReasoning: I added the numerators first",
                "mistake_pattern": "Mixed units",
                "feedback": "Label units before calculating",
                "practice_adjustment": "Add a unit-labeling step",
                "next_exercise": "Try three unit-conversion problems",
                "status": null,
                "capture_memory": true,
            }))
        );
    }

    #[test]
    fn learning_math_record_attempt_maps_to_attempt_patch_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "learning_math_record_attempt",
            &json!({
                "practice_session_id": "session-123",
                "answer": "3/4",
                "reasoning": "I converted both fractions to fourths",
                "capture_memory": true
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Patch);
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/learning/math/practice-sessions/session-123/attempt"
        );
        assert_eq!(
            request.body,
            Some(json!({
                "answer": "3/4\n\nReasoning: I converted both fractions to fourths",
                "capture_memory": true,
            }))
        );
    }

    #[test]
    fn learning_math_record_feedback_maps_to_feedback_patch_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let request = build_api_request_for_tool(
            &config,
            "learning_math_record_feedback",
            &json!({
                "practice_session_id": "session-123",
                "mistake_pattern": "Changed units between steps",
                "feedback": "Write the unit next to every number",
                "practice_adjustment": "Add a unit check before calculating",
                "next_exercise": "Three unit-conversion fraction problems",
                "status": "completed",
                "capture_memory": true
            }),
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Patch);
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/learning/math/practice-sessions/session-123/feedback"
        );
        assert_eq!(
            request.body,
            Some(json!({
                "mistake_pattern": "Changed units between steps",
                "feedback": "Write the unit next to every number",
                "practice_adjustment": "Add a unit check before calculating",
                "next_exercise": "Three unit-conversion fraction problems",
                "status": "completed",
                "capture_memory": true,
            }))
        );
    }

    #[test]
    fn learning_math_list_and_get_practice_sessions_map_to_practice_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let list = build_api_request_for_tool(
            &config,
            "learning_math_list_practice_sessions",
            &json!({
                "space_id": "space-123",
                "namespace_id": "namespace-123",
                "limit": 10,
                "offset": 5
            }),
        )
        .unwrap();
        let get = build_api_request_for_tool(
            &config,
            "learning_math_get_practice_session",
            &json!({
                "practice_session_id": "session-123"
            }),
        )
        .unwrap();

        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/learning/math/practice-sessions?space_id=space-123&namespace_id=namespace-123&limit=10&offset=5"
        );
        assert_eq!(get.method, HttpMethod::Get);
        assert_eq!(
            get.url,
            "http://localhost:8080/api/v1/learning/math/practice-sessions/session-123"
        );
    }
}
