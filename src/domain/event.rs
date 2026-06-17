use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SpaceId;

pub type NamespaceId = Uuid;
pub type TraceId = Uuid;
pub type EnginePayloadId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineEvent {
    ObservationCaptured(EngineEventEnvelope),
    AttemptSubmitted(EngineEventEnvelope),
    FeedbackGenerated(EngineEventEnvelope),
    SleepCycleRequested(EngineEventEnvelope),
    GrowthModelUpdated(EngineEventEnvelope),
    PlanGenerated(EngineEventEnvelope),
}

impl EngineEvent {
    pub fn envelope(&self) -> &EngineEventEnvelope {
        match self {
            Self::ObservationCaptured(envelope)
            | Self::AttemptSubmitted(envelope)
            | Self::FeedbackGenerated(envelope)
            | Self::SleepCycleRequested(envelope)
            | Self::GrowthModelUpdated(envelope)
            | Self::PlanGenerated(envelope) => envelope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineEventEnvelope {
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub source_trace_id: TraceId,
    pub payload_refs: Vec<EnginePayloadRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnginePayloadRef {
    pub kind: EnginePayloadRefKind,
    pub id: EnginePayloadId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnginePayloadRefKind {
    Observation,
    Attempt,
    Feedback,
    SleepCycle,
    GrowthModel,
    PracticePlan,
}
