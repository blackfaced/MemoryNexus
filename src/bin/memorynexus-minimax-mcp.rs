use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const SERVER_NAME: &str = "memorynexus-minimax-mcp-probe";

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<Value>(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(
                    &mut stdout,
                    jsonrpc_error(Value::Null, -32700, error.to_string()),
                )?;
                continue;
            }
        };

        if let Some(response) = handle_request(&request) {
            write_response(&mut stdout, response)?;
        }
    }

    Ok(())
}

fn write_response(stdout: &mut impl Write, response: Value) -> io::Result<()> {
    writeln!(stdout, "{response}")?;
    stdout.flush()
}

fn handle_request(request: &Value) -> Option<Value> {
    let id = request.get("id")?.clone();
    let result = match request.get("method").and_then(Value::as_str) {
        Some("initialize") => Ok(initialize_result()),
        Some("tools/list") => Ok(tools_list_result()),
        Some("tools/call") => call_tool(request.get("params").unwrap_or(&Value::Null)),
        Some(method) => Err(format!("unsupported MCP method: {method}")),
        None => Err("missing MCP method".to_string()),
    };

    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(message) => jsonrpc_error(id, -32601, message),
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")}
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [{
            "name": "memorynexus_status",
            "description": "Read-only MiniMax child-Agent capability probe. It accepts no user content and never reads or writes a ledger.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        }]
    })
}

fn call_tool(params: &Value) -> Result<Value, String> {
    match params.get("name").and_then(Value::as_str) {
        Some("memorynexus_status") => Ok(json!({
            "content": [{
                "type": "text",
                "text": "{\"status\":\"probe_ready\",\"mode\":\"read_only\",\"persistence\":\"none\",\"next_step\":\"Ask the owner to confirm this tool is visible to the MemoryNexus WeChat child Agent.\"}"
            }],
            "structuredContent": {
                "status": "probe_ready",
                "mode": "read_only",
                "persistence": "none"
            }
        })),
        Some(name) => Err(format!("unknown tool: {name}")),
        None => Err("tools/call requires params.name".to_string()),
    }
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_only_the_read_only_probe_tool() {
        let tools = tools_list_result();
        assert_eq!(tools["tools"].as_array().unwrap().len(), 1);
        assert_eq!(tools["tools"][0]["name"], "memorynexus_status");
    }

    #[test]
    fn status_tool_reports_no_persistence() {
        let result = call_tool(&json!({"name": "memorynexus_status"})).unwrap();
        assert_eq!(result["structuredContent"]["status"], "probe_ready");
        assert_eq!(result["structuredContent"]["persistence"], "none");
    }
}
