//! API 路由模块

use axum::Router;

use crate::state::AppState;

mod agent_router;
mod ai;
mod auth;
mod embedding;
mod feedback_loops;
mod health;
mod learning_math;
mod lens_runs;
mod lenses;
mod memories;
mod namespaces;
mod profiles;
mod reminders;
mod review_reports;
mod search;
mod semantic;
mod spaces;
mod surfaces;
mod tags;
mod upload;
mod voice;
mod web;

/// 聚合所有 API 路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(web::routes())
        // 健康检查
        .route("/api/v1/health", axum::routing::get(health::check))
        // 认证
        .route("/api/v1/auth/login", axum::routing::post(auth::login))
        .route("/api/v1/auth/register", axum::routing::post(auth::register))
        .route("/api/v1/auth/me", axum::routing::get(auth::me))
        // Agent routing
        .route(
            "/api/v1/agent/route",
            axum::routing::post(agent_router::route),
        )
        // Surface Gateway
        .route("/api/v1/surfaces", axum::routing::post(surfaces::handle))
        // Cognitive Space
        .route("/api/v1/spaces", axum::routing::get(spaces::list))
        .route("/api/v1/spaces", axum::routing::post(spaces::create))
        .route("/api/v1/spaces/:id", axum::routing::get(spaces::get))
        .route(
            "/api/v1/spaces/:id/members",
            axum::routing::get(spaces::list_members),
        )
        .route(
            "/api/v1/spaces/:id/members/:user_id",
            axum::routing::patch(spaces::update_member_role),
        )
        .route(
            "/api/v1/spaces/:id/invites",
            axum::routing::post(spaces::create_invite),
        )
        .route(
            "/api/v1/spaces/invites/accept",
            axum::routing::post(spaces::accept_invite),
        )
        // Lens
        .route("/api/v1/lenses", axum::routing::get(lenses::list))
        .route("/api/v1/lenses", axum::routing::post(lenses::create))
        .route("/api/v1/lenses/:id", axum::routing::get(lenses::get))
        .route("/api/v1/lens-runs", axum::routing::get(lens_runs::list))
        .route("/api/v1/lens-runs", axum::routing::post(lens_runs::create))
        .route("/api/v1/lens-runs/:id", axum::routing::get(lens_runs::get))
        // Namespace
        .route("/api/v1/namespaces", axum::routing::get(namespaces::list))
        .route(
            "/api/v1/namespaces",
            axum::routing::post(namespaces::create),
        )
        .route(
            "/api/v1/namespaces/:id",
            axum::routing::get(namespaces::get),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/practice-sessions",
            axum::routing::get(learning_math::list_in_namespace),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/practice-sessions",
            axum::routing::post(learning_math::create_in_namespace),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/practice-sessions/:id/attempt",
            axum::routing::patch(learning_math::patch_attempt_in_namespace),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/practice-sessions/:id/feedback",
            axum::routing::patch(learning_math::patch_feedback_in_namespace),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/practice-sessions/:id",
            axum::routing::get(learning_math::get_in_namespace),
        )
        .route(
            "/api/v1/namespaces/:namespace_id/learning-reviews",
            axum::routing::post(review_reports::create_learning_review),
        )
        // FeedbackLoop
        .route(
            "/api/v1/feedback-loops",
            axum::routing::get(feedback_loops::list),
        )
        .route(
            "/api/v1/feedback-loops",
            axum::routing::post(feedback_loops::create),
        )
        .route(
            "/api/v1/feedback-loops/:id",
            axum::routing::get(feedback_loops::get),
        )
        .route(
            "/api/v1/feedback-loops/:id",
            axum::routing::patch(feedback_loops::patch),
        )
        // learning.math practice sessions
        .route(
            "/api/v1/learning/math/practice-sessions",
            axum::routing::get(learning_math::list),
        )
        .route(
            "/api/v1/learning/math/practice-sessions",
            axum::routing::post(learning_math::create),
        )
        .route(
            "/api/v1/learning/math/practice-sessions/:id/attempt",
            axum::routing::patch(learning_math::patch_attempt),
        )
        .route(
            "/api/v1/learning/math/practice-sessions/:id/feedback",
            axum::routing::patch(learning_math::patch_feedback),
        )
        .route(
            "/api/v1/learning/math/practice-sessions/:id",
            axum::routing::get(learning_math::get),
        )
        // 记忆 CRUD
        .route("/api/v1/memories", axum::routing::get(memories::list))
        .route("/api/v1/memories", axum::routing::post(memories::create))
        .route("/api/v1/memories/:id", axum::routing::get(memories::get))
        .route(
            "/api/v1/memories/:id",
            axum::routing::patch(memories::update),
        )
        .route(
            "/api/v1/memories/:id",
            axum::routing::delete(memories::delete),
        )
        .route("/api/v1/voice-captures", axum::routing::post(voice::create))
        // Scheduled recall reminders
        .route("/api/v1/reminders", axum::routing::post(reminders::create))
        .route("/api/v1/reminders", axum::routing::get(reminders::list))
        .route(
            "/api/v1/reminders/:id/complete",
            axum::routing::post(reminders::complete),
        )
        .route(
            "/api/v1/reminders/:id/delivery",
            axum::routing::post(reminders::update_delivery),
        )
        // Cognitive Profile projections
        .route("/api/v1/profiles", axum::routing::post(profiles::create))
        .route("/api/v1/profiles/:id", axum::routing::get(profiles::get))
        // Cognitive Review Reports
        .route(
            "/api/v1/review-reports",
            axum::routing::post(review_reports::create),
        )
        .route(
            "/api/v1/review-reports",
            axum::routing::get(review_reports::list),
        )
        .route(
            "/api/v1/review-reports/:id",
            axum::routing::get(review_reports::get),
        )
        // 标签 CRUD
        .route("/api/v1/tags", axum::routing::get(tags::list))
        .route("/api/v1/tags", axum::routing::post(tags::create))
        .route("/api/v1/tags/:id", axum::routing::get(tags::get))
        .route("/api/v1/tags/:id", axum::routing::patch(tags::update))
        .route("/api/v1/tags/:id", axum::routing::delete(tags::delete))
        // 搜索
        .route("/api/v1/search", axum::routing::get(search::search))
        .route(
            "/api/v1/search/suggest",
            axum::routing::get(search::suggest),
        )
        // AI 功能
        .route("/api/v1/ai/summarize", axum::routing::post(ai::summarize))
        .route("/api/v1/ai/autotag", axum::routing::post(ai::auto_tag))
        .route("/api/v1/ai/config", axum::routing::get(ai::get_config))
        .route(
            "/api/v1/memories/:id/summarize",
            axum::routing::post(ai::summarize_memory),
        )
        // 向量管理
        .route(
            "/api/v1/embeddings",
            axum::routing::post(embedding::create_embedding),
        )
        .route(
            "/api/v1/embeddings/batch",
            axum::routing::post(embedding::batch_create_embeddings),
        )
        .route(
            "/api/v1/embeddings/:id",
            axum::routing::delete(embedding::delete_embeddings),
        )
        .route(
            "/api/v1/embeddings/:id",
            axum::routing::get(embedding::check_embedding),
        )
        // 语义搜索
        .route(
            "/api/v1/search/semantic",
            axum::routing::post(semantic::semantic_search),
        )
        .route(
            "/api/v1/search/semantic",
            axum::routing::get(semantic::semantic_search_get),
        )
    // 文件上传 (预留)
    // .route("/api/v1/upload", axum::routing::post(upload::upload))
    // .route("/api/v1/media/:key", axum::routing::get(upload::get_media))
}

#[cfg(test)]
mod tests {
    #[test]
    fn thought_review_app_uses_user_facing_language() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("写下你现在脑子里最占空间的一件事"));
        assert!(html.contains("不同视角"));
        assert!(html.contains("最近的我在反复想什么"));
        assert!(!html.contains("Add memory"));
    }

    #[test]
    fn thought_review_app_has_memory_list_view() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("data-view=\"memories\""));
        assert!(html.contains("id=\"memoriesView\""));
        assert!(html.contains("还没有保存的想法"));
    }

    #[test]
    fn thought_review_app_lists_memories_with_space_pagination() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("/api/v1/memories?${memoryParams.toString()}"));
        assert!(html.contains("memoryParams.set(\"space_id\", state.space.id)"));
        assert!(html.contains("memoryParams.set(\"limit\", String(state.memories.limit))"));
        assert!(html.contains("memoryParams.set(\"offset\", String(state.memories.offset))"));
        assert!(html.contains("previousMemoriesButton"));
        assert!(html.contains("nextMemoriesButton"));
    }

    #[test]
    fn thought_review_app_exposes_memory_filter_and_sort_controls() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("id=\"memoryFilterTagInput\""));
        assert!(html.contains("id=\"memoryTypeFilterSelect\""));
        assert!(html.contains("id=\"memorySortSelect\""));
        assert!(html.contains("memoryParams.set(\"tag\""));
        assert!(html.contains("memoryParams.set(\"memory_type\""));
        assert!(html.contains("memoryParams.set(\"sort\", state.memories.sort)"));
        assert!(html.contains("applyMemoryFilters"));
        assert!(html.contains("clearMemoryFilters"));
    }

    #[test]
    fn thought_review_app_exposes_active_space_selector() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("id=\"spaceSelect\""));
        assert!(html.contains("id=\"activeSpaceNotice\""));
        assert!(html.contains("state.spaces"));
        assert!(html.contains("switchActiveSpace"));
        assert!(html.contains("你正在保存到"));
    }

    #[test]
    fn thought_review_app_routes_work_to_selected_space() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("setActiveSpace"));
        assert!(html.contains("renderSpaceOptions"));
        assert!(html.contains("/api/v1/lenses?space_id=${state.space.id}"));
        assert!(html.contains("/api/v1/lens-runs?space_id=${state.space.id}&limit=12"));
        assert!(html.contains("space_id: activeSpace.id"));
    }

    #[test]
    fn thought_review_app_shows_space_errors_without_logging_out() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("showSpaceError"));
        assert!(html.contains("spaceError"));
        assert!(html.contains("无法访问当前空间"));
    }

    #[test]
    fn thought_review_app_exposes_generic_lens_run_detail_flow() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("id=\"lensRunSelect\""));
        assert!(html.contains("id=\"lensRunQueryInput\""));
        assert!(html.contains("id=\"runLensButton\""));
        assert!(html.contains("/api/v1/lens-runs"));
        assert!(html.contains("openLensRunDetail"));
        assert!(html.contains("/api/v1/lens-runs/${runId}"));
        assert!(html.contains("summary_provider"));
        assert!(html.contains("summary_source"));
        assert!(html.contains("summary_model"));
        assert!(html.contains("summary_fallback_reason"));
        assert!(html.contains("key_points"));
        assert!(html.contains("open_questions"));
        assert!(html.contains("suggested_next_actions"));
        assert!(html.contains("citations"));
    }

    #[test]
    fn thought_review_app_has_space_scoped_search_view() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("data-view=\"search\""));
        assert!(html.contains("id=\"searchView\""));
        assert!(html.contains("id=\"searchInput\""));
        assert!(html.contains("id=\"searchModeSelect\""));
        assert!(html.contains("searchParams.set(\"space_id\", activeSpace.id)"));
        assert!(html.contains("/api/v1/search?"));
        assert!(html.contains("/api/v1/search/semantic"));
        assert!(html.contains("hydrateSemanticSearchResults"));
        assert!(html.contains("/api/v1/memories/${encodeURIComponent(result.id)}"));
    }

    #[test]
    fn thought_review_app_renders_search_result_provenance_and_provider_errors() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("renderSearchResults"));
        assert!(html.contains("searchResultCard"));
        assert!(html.contains("memory_type"));
        assert!(html.contains("relevance"));
        assert!(html.contains("matched_on"));
        assert!(html.contains("providerFriendlySearchError"));
        assert!(html.contains("Embedding provider 未配置"));
        assert!(html.contains("Qdrant 向量存储未配置"));
        assert!(html.contains("没有找到匹配的想法"));
    }

    #[test]
    fn thought_review_app_has_memory_detail_edit_and_delete_flow() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("data-memory-detail"));
        assert!(html.contains("openMemoryDetail"));
        assert!(html.contains("saveMemoryDetail"));
        assert!(html.contains("deleteMemoryDetail"));
        assert!(html.contains("/api/v1/memories/${encodeURIComponent(memoryId)}"));
        assert!(html.contains("method: \"PATCH\""));
        assert!(html.contains("method: \"DELETE\""));
    }

    #[test]
    fn thought_review_app_sends_empty_title_as_explicit_clear() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("title: titleInput.value.trim()"));
        assert!(!html.contains("title: titleInput.value.trim() || null"));
    }

    #[test]
    fn thought_review_app_has_auth_inline_error_and_pending_state_hooks() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("id=\"authError\""));
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("showAuthError"));
        assert!(html.contains("clearAuthError"));
        assert!(html.contains("setAuthBusy"));
        assert!(html.contains("正在创建..."));
        assert!(html.contains("正在登录..."));
        assert!(html.contains("请输入邮箱"));
        assert!(html.contains("请输入密码"));
        assert!(html.contains("认证失败"));
        assert!(html.contains("邮箱或密码不正确，请检查后再试。"));
    }

    #[test]
    fn thought_review_app_clears_invalid_session_without_clearing_auth_inputs() {
        let html = super::web::thought_review_app_source();

        assert!(html.contains("handleAuthExpired"));
        assert!(html.contains("登录已过期，请重新登录。"));
        assert!(html.contains("response.status === 401"));
        assert!(html.contains("throw new Error(\"登录已过期，请重新登录。\")"));
        assert!(html.contains("clearSession({ preserveAuthInputs: true })"));
        assert!(html.contains("clearSession({ preserveAuthInputs: false })"));
        assert!(!html.contains("$(\"emailInput\").value = \"\""));
        assert!(!html.contains("$(\"passwordInput\").value = \"\""));
    }

    #[test]
    fn learning_stem_app_is_served_as_static_rust_ui() {
        let routes = include_str!("mod.rs");
        let web_routes = include_str!("web.rs");
        let html = super::web::learning_stem_app_source();

        assert!(routes.contains(".merge(web::routes())"));
        assert!(web_routes.contains("\"/learning/stem\""));
        assert!(web_routes.contains("include_str!(\"../../web/learning_stem.html\")"));
        assert!(html.contains("STEM practice"));
        assert!(html.contains("id=\"spaceSelect\""));
        assert!(html.contains("id=\"recentSessions\""));
    }

    #[test]
    fn surface_gateway_route_uses_shared_handle_entrypoint() {
        let routes_source = include_str!("mod.rs");
        let routes = routes_source
            .split("#[cfg(test)]")
            .next()
            .expect("routes source should contain production code before tests");

        assert!(routes.contains("\"/api/v1/surfaces\""));
        assert!(routes.contains("axum::routing::post(surfaces::handle)"));
    }

    #[test]
    fn learning_stem_app_uses_namespace_driven_practice_api() {
        let html = super::web::learning_stem_app_source();

        assert!(html.contains("learning.stem"));
        assert!(html.contains("kind: \"skill\""));
        assert!(html.contains("/api/v1/namespaces?space_id=${encodeURIComponent(space.id)}"));
        assert!(html.contains("/api/v1/namespaces"));
        assert!(html.contains(
            "/api/v1/namespaces/${encodeURIComponent(state.namespace.id)}/practice-sessions"
        ));
        assert!(html.contains("/attempt"));
        assert!(html.contains("/feedback"));
        assert!(!html.contains("/api/v1/learning/math/practice-sessions"));
    }

    #[test]
    fn learning_stem_app_uses_parent_learner_language_and_states() {
        let html = super::web::learning_stem_app_source();

        for token in [
            "practice",
            "answer",
            "mistake pattern",
            "feedback",
            "next exercise",
            "weekly learning review",
            "fraction word problems",
            "Loading",
            "No practice sessions yet",
            "Please choose a Space first",
            "API error",
        ] {
            assert!(html.contains(token), "missing UI token: {token}");
        }

        for backend_term in [
            "MemoryAtom",
            "CognitiveScene",
            "CognitiveProjection",
            "Lens Run",
        ] {
            assert!(
                !html.contains(backend_term),
                "learning.stem UI should not expose backend term {backend_term}"
            );
        }
    }

    #[test]
    fn learning_stem_app_clears_stale_namespace_when_switching_spaces() {
        let html = super::web::learning_stem_app_source();

        assert!(html.contains("resetLearningStemWorkspace"));
        assert!(html.contains("state.namespace = null;"));
        assert!(html.contains("state.currentSession = null;"));
        assert!(html.contains("$(\"attemptPanel\").classList.add(\"hidden\")"));
        assert!(html.contains("$(\"feedbackPanel\").classList.add(\"hidden\")"));
        assert!(html.contains("state.namespace.space_id !== state.space.id"));
        assert!(html.contains("async function switchActiveSpace"));
        assert!(html.contains("switchActiveSpace(event.target.value)"));
    }

    #[test]
    fn learning_stem_recent_sessions_render_practice_adjustment() {
        let html = super::web::learning_stem_app_source();

        assert!(html.contains("practice_adjustment"));
        assert!(html.contains("<strong>practice adjustment:</strong>"));
    }
}
