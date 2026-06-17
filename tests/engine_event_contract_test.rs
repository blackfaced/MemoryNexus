use memorynexus::domain::event::{
    EngineEvent, EngineEventEnvelope, EnginePayloadRef, EnginePayloadRefKind,
};
use serde_json::json;
use uuid::Uuid;

fn envelope(kind: EnginePayloadRefKind) -> EngineEventEnvelope {
    EngineEventEnvelope {
        space_id: Uuid::new_v4(),
        namespace_id: Uuid::new_v4(),
        source_trace_id: Uuid::new_v4(),
        payload_refs: vec![EnginePayloadRef {
            kind,
            id: Uuid::new_v4(),
        }],
    }
}

#[test]
fn engine_events_round_trip_through_json() {
    let events = vec![
        EngineEvent::ObservationCaptured(envelope(EnginePayloadRefKind::Observation)),
        EngineEvent::AttemptSubmitted(envelope(EnginePayloadRefKind::Attempt)),
        EngineEvent::FeedbackGenerated(envelope(EnginePayloadRefKind::Feedback)),
        EngineEvent::SleepCycleRequested(envelope(EnginePayloadRefKind::SleepCycle)),
        EngineEvent::GrowthModelUpdated(envelope(EnginePayloadRefKind::GrowthModel)),
        EngineEvent::PlanGenerated(envelope(EnginePayloadRefKind::PracticePlan)),
    ];

    for event in events {
        let json = serde_json::to_string(&event).expect("engine event should serialize");
        let decoded: EngineEvent =
            serde_json::from_str(&json).expect("engine event should deserialize");

        assert_eq!(decoded, event);
    }
}

#[test]
fn engine_event_json_uses_stable_snake_case_wire_names() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let source_trace_id = Uuid::new_v4();
    let payload_id = Uuid::new_v4();

    let event = EngineEvent::SleepCycleRequested(EngineEventEnvelope {
        space_id,
        namespace_id,
        source_trace_id,
        payload_refs: vec![EnginePayloadRef {
            kind: EnginePayloadRefKind::SleepCycle,
            id: payload_id,
        }],
    });

    let value = serde_json::to_value(event).expect("engine event should serialize");

    assert_eq!(
        value,
        json!({
            "sleep_cycle_requested": {
                "space_id": space_id,
                "namespace_id": namespace_id,
                "source_trace_id": source_trace_id,
                "payload_refs": [
                    {
                        "kind": "sleep_cycle",
                        "id": payload_id
                    }
                ]
            }
        })
    );
}

#[test]
fn engine_event_envelope_carries_routing_trace_and_payload_references() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let source_trace_id = Uuid::new_v4();
    let payload_id = Uuid::new_v4();

    let event = EngineEvent::PlanGenerated(EngineEventEnvelope {
        space_id,
        namespace_id,
        source_trace_id,
        payload_refs: vec![EnginePayloadRef {
            kind: EnginePayloadRefKind::PracticePlan,
            id: payload_id,
        }],
    });

    let envelope = event.envelope();

    assert_eq!(envelope.space_id, space_id);
    assert_eq!(envelope.namespace_id, namespace_id);
    assert_eq!(envelope.source_trace_id, source_trace_id);
    assert_eq!(envelope.payload_refs[0].id, payload_id);
}
