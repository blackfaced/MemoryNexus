use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use memorynexus::{
    reference_adapter::{
        AdapterError, GatewayAcknowledgement, GatewayAcknowledgementStatus, GatewayClient,
        NormalizedGatewayRequest, NormalizedSourceIdentity, Normalizer, ReferenceAdapter,
        SourceClient, SourceRecord, SystemClock,
    },
    study_buddy_adapter::{
        StudyBuddyAdapterConfig, StudyBuddyNormalizer, StudyBuddySourceClient,
        FOUNDATION_CORRECTION_GOAL, FOUNDATION_INITIAL_GOAL, FOUNDATION_REINFORCEMENT_GOAL,
    },
};
use serde_json::{json, Value};
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn source_client_uses_only_the_authenticated_loopback_cursor_feed() {
    async fn feed(headers: HeaderMap, Query(query): Query<HashMap<String, String>>) -> Json<Value> {
        assert_eq!(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer source-token")
        );
        assert_eq!(query.get("after").map(String::as_str), Some("12"));
        assert_eq!(query.get("limit").map(String::as_str), Some("4"));
        assert_eq!(query.get("schemaVersion").map(String::as_str), Some("1"));
        Json(json!({
            "eventSchemaVersion": 1,
            "events": [{
                "sequence": 13,
                "eventId": "00000000-0000-4000-8000-000000000013",
                "eventType": "learning_attempt_recorded",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:00:00Z",
                "subjectRef": "00000000-0000-4000-8000-000000000002",
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": "00000000-0000-4000-8000-000000000001",
                    "recordType": "learning_attempt",
                    "recordId": "attempt:future:1",
                    "revision": 1
                },
                "payload": {
                    "kind": "learning_attempt",
                    "subjectRef": "00000000-0000-4000-8000-000000000002",
                    "attemptRole": "original",
                    "subject": "math",
                    "problem": "7 + 5",
                    "submittedAnswer": "11",
                    "expectedAnswer": "12",
                    "mistakeType": "carry",
                    "isCorrect": false
                },
                "additiveProviderField": "ignored"
            }, {
                "sequence": 14,
                "eventId": "00000000-0000-4000-8000-000000000014",
                "eventType": "learning_session_completed",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:10:00Z",
                "subjectRef": "00000000-0000-4000-8000-000000000002",
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": "00000000-0000-4000-8000-000000000001",
                    "recordType": "learning_session",
                    "recordId": "session:legacy:1",
                    "revision": 1
                },
                "payload": {
                    "kind": "learning_session",
                    "subjectRef": "00000000-0000-4000-8000-000000000002",
                    "sessionKind": "study",
                    "subject": "math",
                    "startedAt": 1786521600000_i64,
                    "endedAt": 1786522200000_i64,
                    "durationMinutes": 10,
                    "averageFocusScore": 80,
                    "postureWarningCount": 0,
                    "offTopicCount": 0,
                    "offTopicRecovered": 0
                }
            }, {
                "sequence": 15,
                "eventId": "00000000-0000-4000-8000-000000000015",
                "eventType": "chat_turn_recorded",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:15:00Z",
                "subjectRef": "00000000-0000-4000-8000-000000000002",
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": "00000000-0000-4000-8000-000000000001",
                    "recordType": "chat_turn",
                    "recordId": "chat_turn:15",
                    "revision": 1
                },
                "payload": {
                    "kind": "chat_turn_reference",
                    "subjectRef": "00000000-0000-4000-8000-000000000002",
                    "sessionRef": "session:bounded-1",
                    "turnRef": "chat_turn:15",
                    "role": "child",
                    "occurredAt": "2026-08-12T08:15:00Z"
                }
            }, {
                "sequence": 16,
                "eventId": "00000000-0000-4000-8000-000000000016",
                "eventType": "source_record_withdrawn",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:16:00Z",
                "subjectRef": "00000000-0000-4000-8000-000000000002",
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": "00000000-0000-4000-8000-000000000001",
                    "recordType": "chat_turn",
                    "recordId": "chat_turn:15",
                    "revision": 2
                },
                "payload": null
            }],
            "page": {
                "after": 12,
                "nextCursor": 16,
                "endOfPage": true,
                "endOfFeed": true,
                "hasMore": false,
                "additivePageField": "ignored"
            }
        }))
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/integration/source-events", get(feed)),
        )
        .await
        .unwrap();
    });
    let client = StudyBuddySourceClient::new(
        &format!("http://{address}/api/integration/source-events"),
        "source-token".to_string(),
        "00000000-0000-4000-8000-000000000002".to_string(),
    )
    .unwrap();

    let page = client.acquire_page(Some("12"), 4).await.unwrap();

    assert_eq!(page.next_cursor.as_deref(), Some("16"));
    assert!(!page.has_more);
    assert_eq!(page.records.len(), 1);
    assert_eq!(
        page.records[0].delivery_key,
        "event:00000000-0000-4000-8000-000000000013"
    );
    let wrong_subject = StudyBuddySourceClient::new(
        &format!("http://{address}/api/integration/source-events"),
        "source-token".to_string(),
        "00000000-0000-4000-8000-000000000003".to_string(),
    )
    .unwrap();
    assert!(wrong_subject.acquire_page(Some("12"), 4).await.is_err());
    assert!(StudyBuddySourceClient::new(
        "https://study-buddy.example/api/integration/source-events",
        "source-token".to_string(),
        "00000000-0000-4000-8000-000000000002".to_string(),
    )
    .is_err());
}

#[tokio::test]
async fn malformed_chat_tombstone_cannot_advance_the_source_cursor() {
    async fn feed() -> Json<Value> {
        Json(json!({
            "eventSchemaVersion": 1,
            "events": [{
                "sequence": 1,
                "eventId": "00000000-0000-4000-8000-000000000011",
                "eventType": "source_record_withdrawn",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:16:00Z",
                "subjectRef": "00000000-0000-4000-8000-000000000002",
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": "00000000-0000-4000-8000-000000000001",
                    "recordType": "chat_turn",
                    "recordId": "chat:not-a-provider-turn-ref",
                    "revision": 2
                },
                "payload": null
            }],
            "page": {
                "after": 0,
                "nextCursor": 1,
                "endOfPage": true,
                "endOfFeed": true,
                "hasMore": false
            }
        }))
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/integration/source-events", get(feed)),
        )
        .await
        .unwrap();
    });
    let client = StudyBuddySourceClient::new(
        &format!("http://{address}/api/integration/source-events"),
        "source-token".to_string(),
        "00000000-0000-4000-8000-000000000002".to_string(),
    )
    .unwrap();

    assert!(client.acquire_page(Some("0"), 1).await.is_err());
}

#[tokio::test]
async fn original_correction_and_reinforcement_are_distinct_deterministic_attempts() {
    let fixture = Fixture::new();
    let normalizer = fixture.normalizer();
    let original = normalizer
        .normalize(fixture.attempt(
            "attempt:original:1",
            1,
            "learning_attempt_recorded",
            "original",
            false,
        ))
        .await
        .expect("original attempt should normalize");
    let correction = normalizer
        .normalize(fixture.attempt(
            "attempt:correction:1",
            1,
            "learning_attempt_recorded",
            "correction",
            true,
        ))
        .await
        .expect("correction attempt should normalize");
    let reinforcement = normalizer
        .normalize(fixture.attempt(
            "attempt:reinforcement:1",
            1,
            "learning_attempt_recorded",
            "reinforcement",
            false,
        ))
        .await
        .expect("reinforcement attempt should normalize");

    let original = serde_json::to_value(original).unwrap();
    let correction = serde_json::to_value(correction).unwrap();
    let reinforcement = serde_json::to_value(reinforcement).unwrap();
    assert_eq!(
        identity(&original, "record_id"),
        json!("attempt:original:1")
    );
    assert_eq!(
        identity(&correction, "record_id"),
        json!("attempt:correction:1")
    );
    assert_eq!(
        identity(&reinforcement, "record_id"),
        json!("attempt:reinforcement:1")
    );
    assert_eq!(evidence(&original, "goal"), json!(FOUNDATION_INITIAL_GOAL));
    assert_eq!(
        evidence(&correction, "goal"),
        json!(FOUNDATION_CORRECTION_GOAL)
    );
    assert_eq!(
        evidence(&reinforcement, "goal"),
        json!(FOUNDATION_REINFORCEMENT_GOAL)
    );
    assert!(evidence(&original, "mistake").is_object());
    assert_eq!(
        evidence(&original, "mistake")["mistake_type"],
        json!("multiplication_fact")
    );
    assert!(evidence(&correction, "mistake").is_null());
    assert!(evidence(&reinforcement, "mistake").is_object());

    let replay = normalizer
        .normalize(fixture.attempt(
            "attempt:original:1",
            1,
            "learning_attempt_recorded",
            "original",
            false,
        ))
        .await
        .unwrap();
    assert_eq!(original, serde_json::to_value(replay).unwrap());
}

#[tokio::test]
async fn source_correction_keeps_identity_and_tombstone_removes_content() {
    let fixture = Fixture::new();
    let normalizer = fixture.normalizer();
    let correction = normalizer
        .normalize(fixture.attempt(
            "attempt:correction:2",
            2,
            "source_record_corrected",
            "correction",
            true,
        ))
        .await
        .unwrap();
    let correction = serde_json::to_value(correction).unwrap();
    assert_eq!(
        identity(&correction, "record_id"),
        json!("attempt:correction:2")
    );
    assert_eq!(identity(&correction, "revision"), json!(2));

    let tombstone = normalizer
        .normalize(fixture.tombstone("attempt:correction:2", 3))
        .await
        .unwrap();
    let tombstone = serde_json::to_value(tombstone).unwrap();
    assert!(tombstone
        .pointer("/payload/source_evidence/evidence")
        .unwrap()
        .is_null());
    assert_eq!(
        tombstone.pointer("/payload/source_evidence/tombstone/reason"),
        Some(&json!("deleted_at_source"))
    );
    assert_eq!(
        identity(&tombstone, "record_id"),
        json!("attempt:correction:2")
    );
    assert_eq!(identity(&tombstone, "revision"), json!(3));

    let mut unscoped_tombstone = fixture.tombstone("attempt:correction:2", 4);
    unscoped_tombstone
        .payload
        .as_object_mut()
        .unwrap()
        .remove("subjectRef");
    assert!(normalizer.normalize(unscoped_tombstone).await.is_err());
}

#[tokio::test]
async fn bounded_sessions_from_the_mixed_feed_use_the_existing_session_variant() {
    let fixture = Fixture::new();
    let normalizer = fixture.normalizer();
    let initial = serde_json::to_value(
        normalizer
            .normalize(fixture.session("session:game:1", None))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(evidence(&initial, "goal"), json!(FOUNDATION_INITIAL_GOAL));
    assert_eq!(evidence(&initial, "kind"), json!("learning_session"));
    assert_eq!(evidence(&initial, "activity_count"), json!(5));
    assert_eq!(evidence(&initial, "successful_activity_count"), json!(4));

    let correction = serde_json::to_value(
        normalizer
            .normalize(fixture.session("session:correction:1", Some("correction")))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        evidence(&correction, "goal"),
        json!(FOUNDATION_CORRECTION_GOAL)
    );
}

#[tokio::test]
async fn unknown_role_or_subject_is_rejected_without_guessing_or_leaking_raw_fields() {
    let fixture = Fixture::new();
    let normalizer = fixture.normalizer();
    let unknown_role = normalizer
        .normalize(fixture.attempt(
            "attempt:unknown:1",
            1,
            "learning_attempt_recorded",
            "teacher_reduced",
            true,
        ))
        .await;
    assert!(unknown_role.is_err());

    let mut unknown_subject = fixture.attempt(
        "attempt:unknown-subject:1",
        1,
        "learning_attempt_recorded",
        "correction",
        true,
    );
    unknown_subject.payload["subjectRef"] = json!(Uuid::new_v4());
    unknown_subject.payload["payload"]["subjectRef"] =
        unknown_subject.payload["subjectRef"].clone();
    assert!(normalizer.normalize(unknown_subject).await.is_err());

    let mut with_provider_extras = fixture.attempt(
        "attempt:extra:1",
        1,
        "learning_attempt_recorded",
        "correction",
        true,
    );
    with_provider_extras.payload["payload"]["mistakeCaseRef"] = json!("case:opaque-1");
    with_provider_extras.payload["payload"]["rawProviderPayload"] =
        json!("ghp_this_must_never_enter_the_normalized_request");
    let normalized = normalizer.normalize(with_provider_extras).await.unwrap();
    let serialized = serde_json::to_string(&normalized).unwrap();
    assert!(!serialized.contains("mistakeCaseRef"));
    assert!(!serialized.contains("rawProviderPayload"));
    assert!(!serialized.contains("ghp_"));

    let mut unknown_mistake_type = fixture.attempt(
        "attempt:unknown-mistake:1",
        1,
        "learning_attempt_recorded",
        "original",
        false,
    );
    unknown_mistake_type.payload["payload"]["mistakeType"] = json!("provider_free_text");
    assert!(normalizer.normalize(unknown_mistake_type).await.is_err());

    assert!(normalizer
        .normalize(fixture.attempt(
            "attempt:invalid-correction-revision",
            1,
            "source_record_corrected",
            "correction",
            true,
        ))
        .await
        .is_err());
    assert!(normalizer
        .normalize(fixture.attempt(
            "attempt:invalid-recorded-revision",
            2,
            "learning_attempt_recorded",
            "original",
            false,
        ))
        .await
        .is_err());
    assert!(normalizer
        .normalize(fixture.tombstone("attempt:invalid-withdrawal-revision", 1))
        .await
        .is_err());
}

#[tokio::test]
async fn mixed_feed_restart_preserves_every_attempt_role_without_persisting_raw_chat() {
    async fn feed(
        State(page): State<Arc<Value>>,
        headers: HeaderMap,
        Query(query): Query<HashMap<String, String>>,
    ) -> Json<Value> {
        assert_eq!(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer source-token")
        );
        assert_eq!(query.get("after").map(String::as_str), Some("0"));
        Json((*page).clone())
    }

    #[derive(Default)]
    struct RestartGateway {
        fail_first_response: AtomicBool,
        deliveries: Mutex<Vec<Value>>,
    }

    #[async_trait]
    impl GatewayClient for RestartGateway {
        async fn deliver(
            &self,
            payload: &NormalizedGatewayRequest,
        ) -> Result<GatewayAcknowledgement, AdapterError> {
            let serialized = serde_json::to_value(payload).unwrap();
            self.deliveries.lock().unwrap().push(serialized.clone());
            if self.fail_first_response.swap(false, Ordering::SeqCst) {
                return Err(AdapterError::Delivery);
            }
            let source_identity: NormalizedSourceIdentity = serde_json::from_value(
                serialized
                    .pointer("/payload/source_evidence/source_identity")
                    .unwrap()
                    .clone(),
            )
            .unwrap();
            Ok(GatewayAcknowledgement {
                status: GatewayAcknowledgementStatus::Accepted,
                source_identity,
            })
        }
    }

    let fixture = Fixture::new();
    let roles = [
        ("attempt:original:restart", "original", false),
        ("attempt:correction:restart:1", "correction", true),
        ("attempt:correction:restart:2", "correction", true),
        ("attempt:reinforcement:restart", "reinforcement", false),
    ];
    let mut events = roles
        .iter()
        .enumerate()
        .map(|(index, (record_id, role, is_correct))| {
            let mut event = fixture
                .attempt(record_id, 1, "learning_attempt_recorded", role, *is_correct)
                .payload;
            event["sequence"] = json!(index + 1);
            event
        })
        .collect::<Vec<_>>();
    events.push(json!({
        "sequence": 5,
        "eventId": Uuid::new_v4(),
        "eventType": "chat_turn_recorded",
        "eventSchemaVersion": 1,
        "occurredAt": "2026-08-12T08:05:00Z",
        "subjectRef": fixture.subject_ref,
        "sourceIdentity": {
            "sourceProduct": "study_buddy",
            "sourceInstallationId": fixture.installation_id,
            "recordType": "chat_turn",
            "recordId": "chat_turn:999",
            "revision": 1
        },
        "payload": {
            "kind": "chat_turn_reference",
            "subjectRef": fixture.subject_ref,
            "sessionRef": "session:bounded-restart",
            "turnRef": "chat_turn:999",
            "role": "child",
            "occurredAt": "2026-08-12T08:05:00Z"
        }
    }));
    let page = Arc::new(json!({
        "eventSchemaVersion": 1,
        "events": events,
        "page": {
            "after": 0,
            "nextCursor": 5,
            "endOfPage": true,
            "endOfFeed": true,
            "hasMore": false
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/api/integration/source-events", get(feed))
                .with_state(page),
        )
        .await
        .unwrap();
    });

    let source = Arc::new(
        StudyBuddySourceClient::new(
            &format!("http://{address}/api/integration/source-events"),
            "source-token".to_string(),
            fixture.subject_ref.to_string(),
        )
        .unwrap(),
    );
    let normalizer = Arc::new(fixture.normalizer());
    let gateway = Arc::new(RestartGateway {
        fail_first_response: AtomicBool::new(true),
        ..RestartGateway::default()
    });
    let directory = tempdir().unwrap();
    let ledger_path = directory.path().join("study-buddy-ledger.db");
    let ledger_url = format!("sqlite://{}", ledger_path.display());

    let first = ReferenceAdapter::open(
        &ledger_url,
        Arc::clone(&source),
        Arc::clone(&normalizer),
        Arc::clone(&gateway),
        Arc::new(SystemClock),
        100,
    )
    .await
    .unwrap();
    assert!(matches!(
        first.run_one_page().await,
        Err(AdapterError::Delivery)
    ));
    drop(first);

    let restarted = ReferenceAdapter::open(
        &ledger_url,
        source,
        normalizer,
        Arc::clone(&gateway),
        Arc::new(SystemClock),
        100,
    )
    .await
    .unwrap();
    let summary = restarted.run_one_page().await.unwrap();
    assert_eq!(
        summary.acquired, 0,
        "the durable page is resumed, not reacquired"
    );
    assert_eq!(summary.acknowledged, 4);
    assert_eq!(summary.cursor.as_deref(), Some("5"));
    drop(restarted);

    let deliveries = gateway.deliveries.lock().unwrap();
    let delivered_ids = deliveries
        .iter()
        .map(|delivery| {
            delivery
                .pointer("/payload/source_evidence/source_identity/record_id")
                .and_then(Value::as_str)
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(delivered_ids.len(), 5, "the lost response is retried once");
    for (record_id, _, _) in roles {
        assert!(delivered_ids.iter().any(|value| value == record_id));
    }
    assert!(!delivered_ids.iter().any(|value| value.contains("chat")));
    drop(deliveries);

    let ledger = std::fs::read(ledger_path).unwrap();
    let ledger = String::from_utf8_lossy(&ledger);
    assert!(!ledger.contains("chat_turn:999"));
}

struct Fixture {
    actor_id: Uuid,
    space_id: Uuid,
    installation_id: Uuid,
    subject_ref: Uuid,
}

impl Fixture {
    fn new() -> Self {
        Self {
            actor_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            installation_id: Uuid::new_v4(),
            subject_ref: Uuid::new_v4(),
        }
    }

    fn normalizer(&self) -> StudyBuddyNormalizer {
        StudyBuddyNormalizer::new(
            StudyBuddyAdapterConfig::new(
                self.actor_id,
                self.space_id,
                HashSet::from([self.subject_ref.to_string()]),
                "0.1.0".to_string(),
            )
            .unwrap(),
        )
    }

    fn attempt(
        &self,
        record_id: &str,
        revision: i64,
        event_type: &str,
        attempt_role: &str,
        is_correct: bool,
    ) -> SourceRecord {
        let event_id = Uuid::new_v4();
        SourceRecord {
            delivery_key: format!("event:{event_id}"),
            payload: json!({
                "sequence": 1,
                "eventId": event_id,
                "eventType": event_type,
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-12T08:00:00Z",
                "subjectRef": self.subject_ref,
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": self.installation_id,
                    "recordType": "learning_attempt",
                    "recordId": record_id,
                    "revision": revision
                },
                "payload": {
                    "kind": "learning_attempt",
                    "subjectRef": self.subject_ref,
                    "attemptRole": attempt_role,
                    "subject": "math",
                    "problem": "3 × 4",
                    "submittedAnswer": if is_correct { "12" } else { "7" },
                    "expectedAnswer": "12",
                    "mistakeType": if is_correct { Value::Null } else { json!("multiply") },
                    "isCorrect": is_correct,
                    "source": "practice"
                }
            }),
        }
    }

    fn tombstone(&self, record_id: &str, revision: i64) -> SourceRecord {
        let event_id = Uuid::new_v4();
        SourceRecord {
            delivery_key: format!("event:{event_id}"),
            payload: json!({
                "sequence": 2,
                "eventId": event_id,
                "eventType": "source_record_withdrawn",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-13T08:00:00Z",
                "subjectRef": self.subject_ref,
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": self.installation_id,
                    "recordType": "learning_attempt",
                    "recordId": record_id,
                    "revision": revision
                },
                "payload": Value::Null
            }),
        }
    }

    fn session(&self, record_id: &str, attempt_role: Option<&str>) -> SourceRecord {
        let event_id = Uuid::new_v4();
        let payload = json!({
            "kind": "learning_session",
            "subjectRef": self.subject_ref,
            "sessionKind": "game",
            "attemptRole": attempt_role.unwrap_or("original"),
            "startedAt": 1_786_348_800_000_i64,
            "endedAt": 1_786_349_700_000_i64,
            "activityCount": 5,
            "successfulActivityCount": 4
        });
        SourceRecord {
            delivery_key: format!("event:{event_id}"),
            payload: json!({
                "sequence": 3,
                "eventId": event_id,
                "eventType": "learning_session_completed",
                "eventSchemaVersion": 1,
                "occurredAt": "2026-08-10T08:15:00Z",
                "subjectRef": self.subject_ref,
                "sourceIdentity": {
                    "sourceProduct": "study_buddy",
                    "sourceInstallationId": self.installation_id,
                    "recordType": "learning_session",
                    "recordId": record_id,
                    "revision": 1
                },
                "payload": payload
            }),
        }
    }
}

fn identity(value: &Value, field: &str) -> Value {
    value
        .pointer(&format!("/payload/source_evidence/source_identity/{field}"))
        .unwrap()
        .clone()
}

fn evidence(value: &Value, field: &str) -> Value {
    value
        .pointer(&format!("/payload/source_evidence/evidence/{field}"))
        .unwrap()
        .clone()
}
