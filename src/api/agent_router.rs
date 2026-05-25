//! Deterministic cognitive router for personal agents

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RouteAgentContextRequest {
    pub message: String,
    pub space_id: Option<Uuid>,
    pub lens_id: Option<Uuid>,
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String {
    "personal_context".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRouteAction {
    WriteMemory,
    SearchMemory,
    RunLens,
    GetProfile,
    Ignore,
}

#[derive(Debug, Serialize)]
pub struct RouteAgentContextResponse {
    pub action: AgentRouteAction,
    pub confidence: f32,
    pub reason_codes: Vec<String>,
    pub safety_flags: Vec<String>,
    pub suggested_tool: Option<String>,
    pub suggested_arguments: Value,
}

/// POST /api/v1/agent/route - Recommend a conservative MemoryNexus action.
pub async fn route(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(mut req): Json<RouteAgentContextRequest>,
) -> Result<Json<ApiResponse<RouteAgentContextResponse>>, AppError> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("message is required".to_string()));
    }

    if let Some(space_id) = req.space_id {
        state
            .repositories
            .spaces
            .find_for_user(space_id, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;
    }

    if let Some(lens_id) = req.lens_id {
        let lens = state
            .repositories
            .lenses
            .find_for_user(lens_id, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;

        if let Some(space_id) = req.space_id {
            if space_id != lens.space_id {
                return Err(AppError::BadRequest(
                    "space_id must match the Lens Cognitive Space".to_string(),
                ));
            }
        } else {
            req.space_id = Some(lens.space_id);
        }
    }

    if req.space_id.is_none() {
        let space = state
            .repositories
            .spaces
            .default_for_user(auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Cognitive space not found".to_string()))?;
        req.space_id = Some(space.id);
    }

    Ok(Json(ApiResponse::success(route_message(&req))))
}

fn route_message(req: &RouteAgentContextRequest) -> RouteAgentContextResponse {
    let message = req.message.trim();
    let lower = message.to_ascii_lowercase();
    let mut reason_codes = Vec::new();
    let mut safety_flags = Vec::new();

    if contains_secret_signal(&lower) {
        reason_codes.push("contains_secret_signal".to_string());
        safety_flags.push("do_not_persist_secret".to_string());
        return response(
            AgentRouteAction::Ignore,
            0.99,
            reason_codes,
            safety_flags,
            None,
            json!({}),
        );
    }

    if looks_like_transient_output(message, &lower) {
        reason_codes.push("transient_or_low_signal".to_string());
        return response(
            AgentRouteAction::Ignore,
            0.86,
            reason_codes,
            safety_flags,
            None,
            json!({}),
        );
    }

    if contains_any(
        &lower,
        &[
            "remember this",
            "please remember",
            "记住",
            "帮我记住",
            "以后记得",
        ],
    ) {
        reason_codes.push("explicit_memory_intent".to_string());
        return response(
            AgentRouteAction::WriteMemory,
            0.92,
            reason_codes,
            safety_flags,
            Some("add_memory"),
            json!({
                "space_id": req.space_id,
                "title": suggested_title(message),
                "content": strip_memory_intent(message),
                "tags": ["agent", "explicit-memory"]
            }),
        );
    }

    if contains_any(
        &lower,
        &[
            "what do you know",
            "what should you know",
            "before helping",
            "context about me",
            "我的背景",
            "了解我",
        ],
    ) {
        reason_codes.push("profile_context_needed".to_string());
        if req.space_id.is_none() {
            reason_codes.push("space_required".to_string());
            return response(
                AgentRouteAction::GetProfile,
                0.7,
                reason_codes,
                safety_flags,
                None,
                json!({}),
            );
        }

        return response(
            AgentRouteAction::GetProfile,
            0.82,
            reason_codes,
            safety_flags,
            Some("get_profile"),
            json!({
                "space_id": req.space_id,
                "lens_id": req.lens_id,
                "target": req.target,
                "limit": 12
            }),
        );
    }

    if contains_any(
        &lower,
        &[
            "review",
            "summarize",
            "analyze",
            "tradeoff",
            "contradiction",
            "risk",
            "复盘",
            "总结",
            "分析",
            "风险",
            "矛盾",
        ],
    ) {
        reason_codes.push("interpretation_needed".to_string());
        let tool = req.lens_id.map(|_| "run_lens");
        return response(
            AgentRouteAction::RunLens,
            if req.lens_id.is_some() { 0.8 } else { 0.64 },
            reason_codes,
            safety_flags,
            tool,
            json!({
                "lens_id": req.lens_id,
                "query": message,
                "limit": 5
            }),
        );
    }

    if is_question(message)
        || contains_any(
            &lower,
            &["find", "search", "recall", "查找", "搜索", "找一下", "记得"],
        )
    {
        reason_codes.push("recall_needed".to_string());
        return response(
            AgentRouteAction::SearchMemory,
            0.72,
            reason_codes,
            safety_flags,
            Some("search_memories"),
            json!({
                "space_id": req.space_id,
                "lens_id": req.lens_id,
                "query": message,
                "semantic": true,
                "limit": 5
            }),
        );
    }

    reason_codes.push("no_durable_memory_action".to_string());
    response(
        AgentRouteAction::Ignore,
        0.61,
        reason_codes,
        safety_flags,
        None,
        json!({}),
    )
}

fn response(
    action: AgentRouteAction,
    confidence: f32,
    reason_codes: Vec<String>,
    safety_flags: Vec<String>,
    suggested_tool: Option<&str>,
    suggested_arguments: Value,
) -> RouteAgentContextResponse {
    RouteAgentContextResponse {
        action,
        confidence,
        reason_codes,
        safety_flags,
        suggested_tool: suggested_tool.map(str::to_string),
        suggested_arguments,
    }
}

fn contains_secret_signal(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "api_key",
            "apikey",
            "secret",
            "password",
            "private key",
            "authorization: bearer",
            "sk-",
            "token=",
            "密码",
            "密钥",
        ],
    )
}

fn looks_like_transient_output(message: &str, lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "stack backtrace",
            "compiling ",
            "finished `",
            "running unittests",
            "error:",
            "warning:",
            "thread '",
        ],
    ) || message.lines().count() > 20
}

fn is_question(message: &str) -> bool {
    let trimmed = message.trim();
    trimmed.ends_with('?') || trimmed.ends_with('？')
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn suggested_title(message: &str) -> String {
    let content = strip_memory_intent(message);
    let mut title = content
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() {
        title = "Agent memory".to_string();
    }
    title
}

fn strip_memory_intent(message: &str) -> String {
    let mut content = message.trim().to_string();
    for prefix in [
        "remember this:",
        "remember this",
        "please remember:",
        "please remember",
        "记住：",
        "记住:",
        "记住",
        "帮我记住：",
        "帮我记住:",
        "帮我记住",
    ] {
        if content.to_ascii_lowercase().starts_with(prefix) {
            content = content[prefix.len()..].trim().to_string();
            break;
        }
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(message: &str) -> RouteAgentContextRequest {
        RouteAgentContextRequest {
            message: message.to_string(),
            space_id: Some(Uuid::new_v4()),
            lens_id: Some(Uuid::new_v4()),
            target: "personal_context".to_string(),
        }
    }

    #[test]
    fn routes_explicit_memory_to_write_memory() {
        let response = route_message(&request("Remember this: I prefer Rust-first projects."));

        assert_eq!(response.action, AgentRouteAction::WriteMemory);
        assert_eq!(response.suggested_tool.as_deref(), Some("add_memory"));
        assert!(response
            .reason_codes
            .contains(&"explicit_memory_intent".to_string()));
    }

    #[test]
    fn routes_questions_to_search() {
        let response = route_message(&request("What did we decide about MemoryNexus phase 3?"));

        assert_eq!(response.action, AgentRouteAction::SearchMemory);
        assert_eq!(response.suggested_tool.as_deref(), Some("search_memories"));
    }

    #[test]
    fn routes_review_requests_to_lens_run() {
        let response = route_message(&request("Review the project risks and contradictions"));

        assert_eq!(response.action, AgentRouteAction::RunLens);
        assert_eq!(response.suggested_tool.as_deref(), Some("run_lens"));
    }

    #[test]
    fn routes_profile_context_requests_to_get_profile() {
        let response = route_message(&request("What should you know about me before helping?"));

        assert_eq!(response.action, AgentRouteAction::GetProfile);
        assert_eq!(response.suggested_tool.as_deref(), Some("get_profile"));
    }

    #[test]
    fn profile_route_without_space_is_not_executable() {
        let response = route_message(&RouteAgentContextRequest {
            message: "What should you know about me before helping?".to_string(),
            space_id: None,
            lens_id: None,
            target: "personal_context".to_string(),
        });

        assert_eq!(response.action, AgentRouteAction::GetProfile);
        assert_eq!(response.suggested_tool, None);
        assert!(response
            .reason_codes
            .contains(&"space_required".to_string()));
    }

    #[test]
    fn never_persists_secret_like_content() {
        let response = route_message(&request("Remember this: OPENAI_API_KEY=sk-secret"));

        assert_eq!(response.action, AgentRouteAction::Ignore);
        assert!(response
            .safety_flags
            .contains(&"do_not_persist_secret".to_string()));
    }

    #[test]
    fn ignores_transient_command_output() {
        let response = route_message(&request("Compiling memorynexus\nwarning: unused import"));

        assert_eq!(response.action, AgentRouteAction::Ignore);
        assert!(response
            .reason_codes
            .contains(&"transient_or_low_signal".to_string()));
    }
}
