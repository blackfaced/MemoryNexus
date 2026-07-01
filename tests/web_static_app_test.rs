#[test]
fn dictation_coach_static_route_and_html_are_registered() {
    let api_routes = include_str!("../src/api/mod.rs");
    let web_routes = include_str!("../src/api/web.rs");
    let html = include_str!("../web/dictation_coach.html");

    assert!(api_routes.contains(".merge(web::routes())"));
    assert!(web_routes.contains("\"/dictation/coach\""));
    assert!(web_routes.contains("include_str!(\"../../web/dictation_coach.html\")"));

    for token in [
        "Dictation Coach",
        "daily dictation",
        "word list",
        "spelling attempt",
        "mistake type",
        "tomorrow practice",
        "7-day trend",
        "/api/v1/surfaces",
        "capture_observation",
        "submit_attempt",
        "generate_next_task",
        "get_state_summary",
        "adapter: \"web\"",
        "runtime_preference: \"deterministic\"",
        "space_id: $(\"spaceInput\").value.trim()",
        "child.chinese.dictation",
        "child.english.spelling",
        "child.english.sentence-dictation",
        "<option value=\"typed\">typed</option>",
        "<option value=\"pasted\">pasted</option>",
        "Ready for typed or pasted daily dictation text.",
    ] {
        assert!(html.contains(token), "missing static app token: {token}");
    }

    for backend_term in [
        "GrowthModel",
        "SleepCycle",
        "MemoryAtom",
        "CognitiveScene",
        "CognitiveProjection",
        "repository row",
        "OCR",
        "ASR",
        "camera",
        "file upload",
        "evidence_refs",
        "input_confirmation",
    ] {
        assert!(
            !html.contains(backend_term),
            "Dictation Coach static app should not expose backend term {backend_term}"
        );
    }
}
