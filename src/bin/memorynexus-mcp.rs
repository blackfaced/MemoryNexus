use memorynexus::domain::evidence::{validate_evidence_request, InputConfirmation};
use serde_json::{json, Map, Value};
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
                "create_practice_session",
                "Create a practice session in a Skill Namespace. This is the canonical namespace-driven practice tool.",
                json!({
                    "type": "object",
                    "properties": {
                        "namespace_id": {"type": "string"},
                        "space_id": {"type": "string", "description": "Optional guard; when supplied it must match the Namespace Space"},
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
                    "required": ["namespace_id", "practice_goal", "exercise"]
                }),
            ),
            tool_schema(
                "record_practice_attempt",
                "Record the learner's answer or reasoning for a namespace-driven practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "namespace_id": {"type": "string"},
                        "practice_session_id": {"type": "string"},
                        "answer": {"type": "string"},
                        "reasoning": {"type": "string"},
                        "capture_memory": {"type": "boolean", "default": false}
                    },
                    "required": ["namespace_id", "practice_session_id"]
                }),
            ),
            tool_schema(
                "record_practice_feedback",
                "Record feedback, mistake pattern, adjustment, and next exercise for a namespace-driven practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "namespace_id": {"type": "string"},
                        "practice_session_id": {"type": "string"},
                        "mistake_pattern": {"type": "string"},
                        "feedback": {"type": "string"},
                        "practice_adjustment": {"type": "string"},
                        "next_exercise": {"type": "string"},
                        "status": {"type": "string", "enum": ["active", "completed", "paused"]},
                        "capture_memory": {"type": "boolean", "default": false}
                    },
                    "required": ["namespace_id", "practice_session_id"]
                }),
            ),
            tool_schema(
                "list_practice_sessions",
                "List practice sessions in a Skill Namespace.",
                json!({
                    "type": "object",
                    "properties": {
                        "namespace_id": {"type": "string"},
                        "space_id": {"type": "string", "description": "Optional guard; when supplied it must match the Namespace Space"},
                        "limit": {"type": "integer", "default": 20},
                        "offset": {"type": "integer", "default": 0}
                    },
                    "required": ["namespace_id"]
                }),
            ),
            tool_schema(
                "get_practice_session",
                "Fetch one namespace-driven practice session.",
                json!({
                    "type": "object",
                    "properties": {
                        "namespace_id": {"type": "string"},
                        "practice_session_id": {"type": "string"}
                    },
                    "required": ["namespace_id", "practice_session_id"]
                }),
            ),
            surface_tool_schema(
                "surface_capture_observation",
                "Capture a generic observation through Surface Gateway.",
                "capture",
                "capture_observation",
            ),
            surface_tool_schema(
                "surface_submit_attempt",
                "Submit a generic performance attempt through Surface Gateway.",
                "performance",
                "submit_attempt",
            ),
            surface_tool_schema(
                "surface_review_evidence",
                "Review generic evidence through Surface Gateway.",
                "reflection",
                "review_evidence",
            ),
            surface_tool_schema(
                "surface_generate_next_task",
                "Generate a generic next task through Surface Gateway.",
                "planning",
                "generate_next_task",
            ),
            surface_tool_schema(
                "surface_adjust_plan",
                "Adjust a generic proposed plan through Surface Gateway.",
                "planning",
                "adjust_plan",
            ),
            surface_tool_schema(
                "surface_get_state_summary",
                "Get a generic namespace state summary through Surface Gateway.",
                "observation",
                "get_state_summary",
            ),
            tool_schema(
                "learning_math_create_practice_session",
                "Compatibility alias: create a parent-assisted learning.math practice session in a Cognitive Space.",
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
                        "checkout_dir": {"type": "string"},
                        "profile": {"type": "string", "enum": memorynexus::install::profile_enum_json()},
                        "release_tag": {"type": "string"},
                        "bin_dir": {"type": "string"},
                        "binary_path": {"type": "string"}
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
                        "profile": {"type": "string", "enum": memorynexus::install::profile_enum_json(), "default": "developer"},
                        "release_tag": {"type": "string"},
                        "bin_dir": {"type": "string"},
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

fn surface_tool_schema(name: &str, description: &str, surface: &str, action: &str) -> Value {
    tool_schema(
        name,
        description,
        json!({
            "type": "object",
            "properties": {
                "namespace": {"type": "string"},
                "actor": {"type": "string", "description": "Authenticated actor UUID"},
                "payload": {
                    "type": "object",
                    "description": "Generic Surface payload. Performance, Reflection, Planning, and Observation payloads must include space_id. Confirmed text is canonical; media descriptors are allowed only for confirmed media-derived Capture/Performance calls."
                },
                "context": {
                    "type": "object",
                    "properties": {
                        "mode": {"type": "string", "enum": ["fast", "focused", "deep", "none"]},
                        "locale": {"type": "string"},
                        "device": {"type": "string"},
                        "runtime_preference": {"type": "string", "enum": ["auto", "cloud", "deterministic", "hybrid", "local"]}
                    }
                },
                "surface": {"type": "string", "const": surface},
                "action": {"type": "string", "const": action}
            },
            "required": ["namespace", "actor"]
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceToolSpec {
    name: &'static str,
    surface: &'static str,
    action: &'static str,
    accepts_evidence_refs: bool,
}

const SURFACE_TOOL_SPECS: &[SurfaceToolSpec] = &[
    SurfaceToolSpec {
        name: "surface_capture_observation",
        surface: "capture",
        action: "capture_observation",
        accepts_evidence_refs: true,
    },
    SurfaceToolSpec {
        name: "surface_submit_attempt",
        surface: "performance",
        action: "submit_attempt",
        accepts_evidence_refs: true,
    },
    SurfaceToolSpec {
        name: "surface_review_evidence",
        surface: "reflection",
        action: "review_evidence",
        accepts_evidence_refs: false,
    },
    SurfaceToolSpec {
        name: "surface_generate_next_task",
        surface: "planning",
        action: "generate_next_task",
        accepts_evidence_refs: false,
    },
    SurfaceToolSpec {
        name: "surface_adjust_plan",
        surface: "planning",
        action: "adjust_plan",
        accepts_evidence_refs: false,
    },
    SurfaceToolSpec {
        name: "surface_get_state_summary",
        surface: "observation",
        action: "get_state_summary",
        accepts_evidence_refs: false,
    },
];

fn surface_tool_spec(tool_name: &str) -> Option<SurfaceToolSpec> {
    SURFACE_TOOL_SPECS
        .iter()
        .copied()
        .find(|spec| spec.name == tool_name)
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

    if let Some(spec) = surface_tool_spec(tool_name) {
        return build_surface_api_request(base_url, token, spec, arguments);
    }

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
        "create_practice_session" => {
            let namespace_id = required_string(arguments, "namespace_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Post,
                url: format!("{base_url}/api/v1/namespaces/{namespace_id}/practice-sessions"),
                body: Some(json!({
                    "space_id": optional_string(arguments, "space_id"),
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
            })
        }
        "record_practice_attempt" => {
            let namespace_id = required_string(arguments, "namespace_id")?;
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Patch,
                url: format!(
                    "{base_url}/api/v1/namespaces/{namespace_id}/practice-sessions/{session_id}/attempt"
                ),
                body: Some(json!({
                    "answer": combined_answer(arguments),
                    "capture_memory": optional_bool(arguments, "capture_memory").unwrap_or(false),
                })),
                token,
            })
        }
        "record_practice_feedback" => {
            let namespace_id = required_string(arguments, "namespace_id")?;
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Patch,
                url: format!(
                    "{base_url}/api/v1/namespaces/{namespace_id}/practice-sessions/{session_id}/feedback"
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
        "list_practice_sessions" => {
            let namespace_id = required_string(arguments, "namespace_id")?;
            let limit = optional_usize(arguments, "limit").unwrap_or(20).to_string();
            let offset = optional_usize(arguments, "offset").unwrap_or(0).to_string();
            let space_id = optional_string(arguments, "space_id");
            let mut pairs = vec![("limit", limit.as_str()), ("offset", offset.as_str())];
            if let Some(space_id) = space_id.as_deref() {
                pairs.insert(0, ("space_id", space_id));
            }
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: with_query(
                    &format!("{base_url}/api/v1/namespaces/{namespace_id}/practice-sessions"),
                    &pairs,
                )?,
                body: None,
                token,
            })
        }
        "get_practice_session" => {
            let namespace_id = required_string(arguments, "namespace_id")?;
            let session_id = required_string(arguments, "practice_session_id")?;
            Ok(ApiRequest {
                method: HttpMethod::Get,
                url: format!(
                    "{base_url}/api/v1/namespaces/{namespace_id}/practice-sessions/{session_id}"
                ),
                body: None,
                token,
            })
        }
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

fn build_surface_api_request(
    base_url: &str,
    token: String,
    spec: SurfaceToolSpec,
    arguments: &Value,
) -> Result<ApiRequest, McpError> {
    let payload = arguments
        .get("payload")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !payload.is_object() {
        return Err(McpError::new("payload must be a JSON object"));
    }
    validate_surface_payload(spec, &payload)?;

    Ok(ApiRequest {
        method: HttpMethod::Post,
        url: format!("{base_url}/api/v1/surfaces"),
        body: Some(json!({
            "namespace": required_string(arguments, "namespace")?,
            "surface": spec.surface,
            "action": spec.action,
            "actor": required_string(arguments, "actor")?,
            "adapter": "mcp",
            "payload": payload,
            "context": surface_context(arguments),
        })),
        token,
    })
}

fn validate_surface_payload(spec: SurfaceToolSpec, payload: &Value) -> Result<(), McpError> {
    let source = surface_payload_source(payload);
    if matches!(source.as_deref(), Some("typed" | "pasted")) {
        reject_media_only_fields_for_text_source(payload)?;
    }

    if payload.get("evidence_refs").is_some() && !spec.accepts_evidence_refs {
        return Err(McpError::new(
            "evidence_refs are supported only for confirmed media-derived Capture/Performance Surface calls",
        ));
    }
    if payload.get("evidence_refs").is_some()
        && !matches!(
            source.as_deref(),
            Some("agent_ocr" | "agent_transcribed" | "mixed")
        )
    {
        return Err(McpError::new(
            "evidence_refs require a confirmed media-derived source",
        ));
    }
    if let Some(evidence_refs) = payload.get("evidence_refs") {
        if !evidence_refs.is_array() {
            return Err(McpError::new("evidence_refs must be a JSON array"));
        }
    }

    let confirmation = payload
        .get("input_confirmation")
        .map(|value| serde_json::from_value::<InputConfirmation>(value.clone()))
        .transpose()
        .map_err(|_| McpError::new("invalid_input_confirmation"))?;
    let evidence_refs = payload
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(Vec::as_slice);

    validate_evidence_request(source.as_deref(), confirmation.as_ref(), evidence_refs)
        .map_err(|error| McpError::new(error.to_string()))?;

    Ok(())
}

fn surface_payload_source(payload: &Value) -> Option<String> {
    optional_string(payload, "input_source").or_else(|| optional_string(payload, "source"))
}

fn reject_media_only_fields_for_text_source(payload: &Value) -> Result<(), McpError> {
    for field in MEDIA_ONLY_PAYLOAD_FIELDS {
        if payload.get(field).is_some() {
            return Err(McpError::new(format!(
                "typed/pasted Surface payloads must not include media-only field: {field}"
            )));
        }
    }
    Ok(())
}

const MEDIA_ONLY_PAYLOAD_FIELDS: &[&str] = &[
    "evidence_refs",
    "input_confirmation",
    "provider",
    "locator",
    "media_type",
    "content_hash",
    "original_name",
    "captured_at",
    "transcript",
    "transcript_source",
    "media_descriptor",
    "media_provenance",
];

fn surface_context(arguments: &Value) -> Value {
    let context = arguments.get("context").unwrap_or(&Value::Null);
    json!({
        "mode": optional_string(context, "mode"),
        "locale": optional_string(context, "locale"),
        "device": optional_string(context, "device"),
        "runtime_preference": optional_string(context, "runtime_preference"),
    })
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
    let response = if surface_tool_spec(tool_name).is_some() {
        shape_surface_tool_response(response)
    } else {
        response
    };

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

fn shape_surface_tool_response(response: Value) -> Value {
    remove_descriptor_fields(response)
}

fn remove_descriptor_fields(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(remove_descriptor_fields).collect())
        }
        Value::Object(object) => {
            let mut sanitized = Map::new();
            for (key, value) in object {
                if RESPONSE_DESCRIPTOR_FIELDS.contains(&key.as_str()) {
                    continue;
                }
                sanitized.insert(key, remove_descriptor_fields(value));
            }
            Value::Object(sanitized)
        }
        other => other,
    }
}

const RESPONSE_DESCRIPTOR_FIELDS: &[&str] = &[
    "evidence_refs",
    "input_confirmation",
    "input_source",
    "provider",
    "locator",
    "media_type",
    "content_hash",
    "original_name",
    "captured_at",
    "transcript",
    "transcript_source",
    "media_descriptor",
    "media_provenance",
];

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
            let checkout = optional_string(arguments, "checkout_dir")
                .as_deref()
                .map(|path| resolve_checkout_dir(Some(path)))
                .transpose()?
                .map(|checkout| checkout_status(&checkout));
            let profile = optional_profile(arguments, "profile")?;
            let api_health = fetch_api_health(config).await.unwrap_or_else(|error| {
                json!({
                    "reachable": false,
                    "error": error.message,
                })
            });
            let status = memorynexus::install::install_status_value(
                memorynexus::install::InstallStatusInput {
                    selected_profile: profile,
                    api_url: config.api_url.clone(),
                    api_health,
                    local: local_version_data(),
                    checkout,
                    release_tag: optional_string(arguments, "release_tag"),
                    bin_dir: optional_string(arguments, "bin_dir"),
                    binary_path: optional_string(arguments, "binary_path"),
                    target: memorynexus::install::ReleaseTarget::detect(),
                },
            );

            Ok(status)
        }
        "upgrade_install" => {
            let checkout = optional_string(arguments, "checkout_dir")
                .as_deref()
                .map(|path| resolve_checkout_dir(Some(path)))
                .transpose()?;
            let profile = optional_profile(arguments, "profile")?
                .unwrap_or(memorynexus::install::InstallProfile::Developer);
            let apply = optional_bool(arguments, "apply").unwrap_or(false);
            let pull = optional_bool(arguments, "pull").unwrap_or(false);
            let rebuild_mcp = optional_bool(arguments, "rebuild_mcp").unwrap_or(false);
            let rebuild_api = optional_bool(arguments, "rebuild_api").unwrap_or(false);
            let skip_tests = optional_bool(arguments, "skip_tests").unwrap_or(false);
            let plan = upgrade_plan_value(
                &config.api_url,
                profile,
                pull,
                rebuild_mcp,
                !skip_tests,
                rebuild_api,
            );

            if !apply {
                return Ok(json!({
                    "mode": "plan",
                    "profile": profile.as_str(),
                    "checkout": checkout
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    "plan": plan,
                    "apply_hint": "call upgrade_install with apply=true to execute these local commands",
                }));
            }

            if profile != memorynexus::install::InstallProfile::Developer {
                return Err(McpError::new(
                    "apply=true currently executes only Developer Profile source-build steps; use the binary-first plan commands for this profile or choose profile=developer explicitly",
                ));
            }

            let checkout = checkout.ok_or_else(|| {
                McpError::new(
                    "checkout_dir is required when applying Developer Profile source-build steps",
                )
            })?;

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
                "profile": profile.as_str(),
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

fn optional_profile(
    arguments: &Value,
    key: &str,
) -> Result<Option<memorynexus::install::InstallProfile>, McpError> {
    optional_string(arguments, key)
        .map(|value| memorynexus::install::InstallProfile::parse(&value).map_err(McpError::new))
        .transpose()
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

fn upgrade_plan_value(
    api_url: &str,
    profile: memorynexus::install::InstallProfile,
    pull: bool,
    rebuild_mcp: bool,
    run_tests: bool,
    rebuild_api: bool,
) -> Value {
    memorynexus::install::install_plan_value(
        profile,
        memorynexus::install::InstallPlanOptions::new(
            api_url,
            None,
            None,
            memorynexus::install::ReleaseTarget::detect(),
        )
        .with_source_flags(pull, rebuild_mcp, run_tests, rebuild_api),
    )
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
                "create_practice_session",
                "record_practice_attempt",
                "record_practice_feedback",
                "list_practice_sessions",
                "get_practice_session",
                "surface_capture_observation",
                "surface_submit_attempt",
                "surface_review_evidence",
                "surface_generate_next_task",
                "surface_adjust_plan",
                "surface_get_state_summary",
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

    fn config_with_token() -> Config {
        Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        }
    }

    fn base_surface_args(payload: Value) -> Value {
        json!({
            "namespace": "child.english.spelling",
            "actor": "00000000-0000-0000-0000-000000000001",
            "payload": payload,
            "context": {
                "mode": "fast",
                "locale": "en-US",
                "device": "desktop",
                "runtime_preference": "deterministic"
            }
        })
    }

    #[test]
    fn surface_tools_map_generic_actions_to_surface_gateway() {
        let config = config_with_token();
        let cases = [
            (
                "surface_capture_observation",
                "capture",
                "capture_observation",
                json!({"source": "typed", "content": "because\nfriend"}),
            ),
            (
                "surface_submit_attempt",
                "performance",
                "submit_attempt",
                json!({
                    "space_id": "22222222-2222-2222-2222-222222222222",
                    "source": "pasted",
                    "attempt": {"target": "because", "submitted": "becuase"}
                }),
            ),
            (
                "surface_review_evidence",
                "reflection",
                "review_evidence",
                json!({
                    "space_id": "22222222-2222-2222-2222-222222222222",
                    "question": "What changed?",
                    "evidence": []
                }),
            ),
            (
                "surface_generate_next_task",
                "planning",
                "generate_next_task",
                json!({
                    "space_id": "22222222-2222-2222-2222-222222222222",
                    "objective": "Review spelling pattern"
                }),
            ),
            (
                "surface_adjust_plan",
                "planning",
                "adjust_plan",
                json!({
                    "space_id": "22222222-2222-2222-2222-222222222222",
                    "proposed_plan": {"title": "Draft practice"},
                    "evidence": [],
                    "constraints": ["keep it short"]
                }),
            ),
            (
                "surface_get_state_summary",
                "observation",
                "get_state_summary",
                json!({
                    "space_id": "22222222-2222-2222-2222-222222222222",
                    "timeframe": "7d"
                }),
            ),
        ];

        for (tool, surface, action, payload) in cases {
            let expected_payload = payload.clone();
            let request = build_api_request_for_tool(&config, tool, &base_surface_args(payload))
                .expect("surface tool should build request");

            assert_eq!(request.method, HttpMethod::Post);
            assert_eq!(request.url, "http://localhost:8080/api/v1/surfaces");
            assert_eq!(request.token, "jwt-token");
            assert_eq!(
                request.body.unwrap(),
                json!({
                    "namespace": "child.english.spelling",
                    "surface": surface,
                    "action": action,
                    "actor": "00000000-0000-0000-0000-000000000001",
                    "adapter": "mcp",
                    "payload": expected_payload,
                    "context": {
                        "mode": "fast",
                        "locale": "en-US",
                        "device": "desktop",
                        "runtime_preference": "deterministic"
                    }
                })
            );
        }
    }

    #[test]
    fn typed_or_pasted_surface_payloads_reject_media_only_fields_before_request_build() {
        let config = config_with_token();
        for source in ["typed", "pasted"] {
            for field in [
                "evidence_refs",
                "input_confirmation",
                "provider",
                "locator",
                "media_type",
                "content_hash",
                "original_name",
                "captured_at",
                "transcript",
                "transcript_source",
                "media_descriptor",
                "media_provenance",
            ] {
                let mut payload = json!({"source": source, "content": "because"});
                payload[field] = json!("not allowed");
                let error = build_api_request_for_tool(
                    &config,
                    "surface_capture_observation",
                    &base_surface_args(payload),
                )
                .unwrap_err();

                assert!(
                    error.message.contains("typed/pasted"),
                    "unexpected error for {source} {field}: {}",
                    error.message
                );
                assert!(
                    error.message.contains(field),
                    "error should name rejected field {field}: {}",
                    error.message
                );
            }
        }
    }

    #[test]
    fn media_derived_surface_payloads_require_confirmed_input_confirmation() {
        let config = config_with_token();
        for source in ["agent_ocr", "agent_transcribed", "mixed"] {
            let error = build_api_request_for_tool(
                &config,
                "surface_submit_attempt",
                &base_surface_args(json!({"source": source, "attempt": "becuase"})),
            )
            .unwrap_err();
            assert!(error.message.contains("input_confirmation"));

            for confirmation in [
                json!({"status": "pending", "method": "explicit_acceptance"}),
                json!({"status": "confirmed", "method": "implicit_acceptance"}),
            ] {
                let error = build_api_request_for_tool(
                    &config,
                    "surface_submit_attempt",
                    &base_surface_args(json!({
                        "source": source,
                        "attempt": "becuase",
                        "input_confirmation": confirmation
                    })),
                )
                .unwrap_err();
                assert!(error.message.contains("invalid_input_confirmation"));
            }
        }
    }

    #[test]
    fn confirmed_media_derived_capture_and_performance_accept_evidence_refs() {
        let config = config_with_token();
        let evidence_ref = json!({
            "provider": "agent_ocr",
            "locator": "s3://study/archive/dictation-1.png",
            "media_type": "image/png",
            "metadata": {"page": 1}
        });

        for (tool, payload) in [
            (
                "surface_capture_observation",
                json!({
                    "input_source": "agent_ocr",
                    "content": "because",
                    "input_confirmation": {
                        "status": "confirmed",
                        "method": "explicit_acceptance"
                    },
                    "evidence_refs": [evidence_ref.clone()]
                }),
            ),
            (
                "surface_submit_attempt",
                json!({
                    "source": "mixed",
                    "attempt": "becuase",
                    "input_confirmation": {
                        "status": "confirmed",
                        "method": "explicit_correction"
                    },
                    "evidence_refs": [evidence_ref.clone()]
                }),
            ),
        ] {
            let request = build_api_request_for_tool(&config, tool, &base_surface_args(payload))
                .expect("confirmed media-derived request should build");
            assert_eq!(
                request.body.unwrap()["payload"]["evidence_refs"],
                json!([evidence_ref])
            );
        }
    }

    #[test]
    fn evidence_refs_are_rejected_for_non_capture_performance_surface_tools() {
        let config = config_with_token();
        let error = build_api_request_for_tool(
            &config,
            "surface_review_evidence",
            &base_surface_args(json!({
                "source": "agent_ocr",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_acceptance"
                },
                "evidence_refs": [{
                    "provider": "agent_ocr",
                    "locator": "s3://study/archive/dictation-1.png",
                    "media_type": "image/png",
                    "metadata": {}
                }]
            })),
        )
        .unwrap_err();

        assert!(error.message.contains("Capture/Performance"));
    }

    #[test]
    fn evidence_refs_require_media_derived_source_even_on_capture_performance() {
        let config = config_with_token();
        let error = build_api_request_for_tool(
            &config,
            "surface_capture_observation",
            &base_surface_args(json!({
                "content": "because",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_acceptance"
                },
                "evidence_refs": [{
                    "provider": "agent_ocr",
                    "locator": "s3://study/archive/dictation-1.png",
                    "media_type": "image/png",
                    "metadata": {}
                }]
            })),
        )
        .unwrap_err();

        assert!(error.message.contains("media-derived source"));
    }

    #[test]
    fn surface_tool_response_keeps_trace_provenance_without_descriptor_objects() {
        let api_response = json!({
            "data": {
                "surface": "capture",
                "action": "capture_observation",
                "generated_trace_id": "11111111-1111-1111-1111-111111111111",
                "result": {
                    "status": "captured",
                    "evidence_refs": [{
                        "provider": "agent_ocr",
                        "locator": "s3://private/raw.png",
                        "media_type": "image/png",
                        "metadata": {"secret": "hidden"}
                    }]
                }
            }
        });

        let shaped = shape_surface_tool_response(api_response);
        let text = shaped.to_string();

        assert_eq!(
            shaped["data"]["generated_trace_id"],
            "11111111-1111-1111-1111-111111111111"
        );
        for forbidden in [
            "evidence_refs",
            "locator",
            "provider",
            "metadata",
            "raw.png",
        ] {
            assert!(
                !text.contains(forbidden),
                "surface MCP response leaked {forbidden}: {text}"
            );
        }
    }

    #[test]
    fn upgrade_plan_defaults_to_safe_plan_only_steps() {
        let plan = upgrade_plan_value(
            DEFAULT_API_URL,
            memorynexus::install::InstallProfile::Developer,
            false,
            false,
            true,
            false,
        );
        let steps = plan["steps"].as_array().unwrap();

        assert_eq!(steps[0]["command"], "git status --short");
        assert!(steps.iter().any(|step| step["command"] == "cargo test"));
        assert!(steps
            .iter()
            .any(|step| step["command"] == "restart API and reload MCP client"));
    }

    #[test]
    fn mcp_install_tools_schema_accepts_profile_and_binary_options() {
        let result = tools_list_result();
        let tools = result["tools"].as_array().unwrap();
        let get_status = tools
            .iter()
            .find(|tool| tool["name"] == "get_install_status")
            .unwrap();
        let upgrade = tools
            .iter()
            .find(|tool| tool["name"] == "upgrade_install")
            .unwrap();

        assert_eq!(
            get_status["inputSchema"]["properties"]["profile"]["enum"],
            json!(["trial", "local-one-click", "production", "developer"])
        );
        assert!(get_status["inputSchema"]["properties"]["release_tag"].is_object());
        assert!(get_status["inputSchema"]["properties"]["bin_dir"].is_object());
        assert_eq!(
            upgrade["inputSchema"]["properties"]["profile"]["enum"],
            json!(["trial", "local-one-click", "production", "developer"])
        );
    }

    #[test]
    fn mcp_trial_profile_plan_avoids_cargo_and_local_services() {
        let plan = memorynexus::install::install_plan_value(
            memorynexus::install::InstallProfile::Trial,
            memorynexus::install::InstallPlanOptions::for_test("v0.1.0", "aarch64-apple-darwin"),
        );
        let text = plan.to_string();

        assert!(text.contains("memorynexus-mcp"));
        assert!(text.contains("initialize"));
        assert!(text.contains("tools/list"));
        assert!(!text.contains("cargo"));
        assert!(!text.contains("Docker"));
        assert!(!text.contains("PostgreSQL"));
        assert!(!text.contains("Qdrant"));
    }

    #[test]
    fn mcp_local_one_click_plan_includes_archive_checksum_services_and_smoke() {
        let plan = memorynexus::install::install_plan_value(
            memorynexus::install::InstallProfile::LocalOneClick,
            memorynexus::install::InstallPlanOptions::for_test(
                "v0.1.0",
                "x86_64-unknown-linux-gnu",
            ),
        );
        let text = plan.to_string();

        assert!(text.contains("memorynexus-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"));
        assert!(text.contains("sha256"));
        assert!(text.contains("install.sh --start-services"));
        assert!(text.contains("--print-mcp-config"));
        assert!(text.contains("README.local-one-click.md"));
        assert!(text.contains("memorynexus-cli health"));
        assert!(text.contains("tools/list"));
        assert!(!text.contains("cargo"));
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
                "profile": "developer",
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

    #[tokio::test]
    async fn upgrade_install_trial_plan_uses_configured_api_url() {
        let config = Config {
            api_url: "https://demo.example.test".to_string(),
            token: None,
        };

        let result = call_local_tool(
            &config,
            "upgrade_install",
            &json!({
                "apply": false,
                "profile": "trial"
            }),
        )
        .await
        .unwrap();
        let text = result["plan"].to_string();

        assert!(text.contains("https://demo.example.test"));
        assert!(!text.contains("MEMORYNEXUS_API_URL=http://localhost:8080"));
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
    fn tools_list_exposes_canonical_namespace_practice_tools() {
        let result = tools_list_result();
        let names: Vec<&str> = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();

        for name in [
            "create_practice_session",
            "record_practice_attempt",
            "record_practice_feedback",
            "list_practice_sessions",
            "get_practice_session",
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
    fn canonical_practice_tools_map_to_namespace_practice_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let create = build_api_request_for_tool(
            &config,
            "create_practice_session",
            &json!({
                "namespace_id": "namespace-123",
                "practice_goal": "Improve fraction word problems",
                "exercise": "Solve five fraction word problems",
                "capture_memory": true
            }),
        )
        .unwrap();
        let attempt = build_api_request_for_tool(
            &config,
            "record_practice_attempt",
            &json!({
                "namespace_id": "namespace-123",
                "practice_session_id": "session-123",
                "answer": "3/4"
            }),
        )
        .unwrap();
        let feedback = build_api_request_for_tool(
            &config,
            "record_practice_feedback",
            &json!({
                "namespace_id": "namespace-123",
                "practice_session_id": "session-123",
                "mistake_pattern": "Changed units between steps"
            }),
        )
        .unwrap();
        let list = build_api_request_for_tool(
            &config,
            "list_practice_sessions",
            &json!({
                "namespace_id": "namespace-123",
                "limit": 10,
                "offset": 5
            }),
        )
        .unwrap();
        let get = build_api_request_for_tool(
            &config,
            "get_practice_session",
            &json!({
                "namespace_id": "namespace-123",
                "practice_session_id": "session-123"
            }),
        )
        .unwrap();

        assert_eq!(create.method, HttpMethod::Post);
        assert_eq!(
            create.url,
            "http://localhost:8080/api/v1/namespaces/namespace-123/practice-sessions"
        );
        assert_eq!(attempt.method, HttpMethod::Patch);
        assert_eq!(
            attempt.url,
            "http://localhost:8080/api/v1/namespaces/namespace-123/practice-sessions/session-123/attempt"
        );
        assert_eq!(feedback.method, HttpMethod::Patch);
        assert_eq!(
            feedback.url,
            "http://localhost:8080/api/v1/namespaces/namespace-123/practice-sessions/session-123/feedback"
        );
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/namespaces/namespace-123/practice-sessions?limit=10&offset=5"
        );
        assert_eq!(get.method, HttpMethod::Get);
        assert_eq!(
            get.url,
            "http://localhost:8080/api/v1/namespaces/namespace-123/practice-sessions/session-123"
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
