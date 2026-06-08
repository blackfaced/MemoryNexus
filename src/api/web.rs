//! Static Thought Review UI.

use axum::{response::Html, routing::get, Router};

use crate::state::AppState;

const THOUGHT_REVIEW_APP: &str = include_str!("../../web/thought_review.html");
const LEARNING_STEM_APP: &str = include_str!("../../web/learning_stem.html");

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(app))
        .route("/app", get(app))
        .route("/learning/stem", get(learning_stem_app))
}

async fn app() -> Html<&'static str> {
    Html(thought_review_app_source())
}

async fn learning_stem_app() -> Html<&'static str> {
    Html(learning_stem_app_source())
}

pub(crate) fn thought_review_app_source() -> &'static str {
    THOUGHT_REVIEW_APP
}

pub(crate) fn learning_stem_app_source() -> &'static str {
    LEARNING_STEM_APP
}
