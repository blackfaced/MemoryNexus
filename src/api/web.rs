//! Static Thought Review UI.

use axum::{response::Html, routing::get, Router};

use crate::state::AppState;

const THOUGHT_REVIEW_APP: &str = include_str!("../../web/thought_review.html");

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(app)).route("/app", get(app))
}

async fn app() -> Html<&'static str> {
    Html(thought_review_app_source())
}

pub(crate) fn thought_review_app_source() -> &'static str {
    THOUGHT_REVIEW_APP
}
