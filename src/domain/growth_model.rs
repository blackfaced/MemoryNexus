use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{MemoryId, SpaceId};

pub type GrowthModelId = Uuid;
pub type NamespaceId = Uuid;
pub type TraceId = Uuid;
pub type FeedbackLoopId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EvidenceId {
    Trace(TraceId),
    FeedbackLoop(FeedbackLoopId),
    Memory(MemoryId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedClaim {
    pub label: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedPattern {
    pub pattern: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthStage {
    pub label: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedFocus {
    pub focus: String,
    pub rationale: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthModel {
    pub id: GrowthModelId,
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub strengths: Vec<EvidenceBackedClaim>,
    pub weaknesses: Vec<EvidenceBackedClaim>,
    pub recurring_patterns: Vec<EvidenceBackedPattern>,
    pub current_stage: GrowthStage,
    pub recommended_focus: EvidenceBackedFocus,
    pub updated_at: DateTime<Utc>,
}
