use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{MemoryId, SpaceId};

pub type MemoryAtomId = Uuid;
pub type NamespaceId = Uuid;
pub type TraceId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAtomKind {
    Observation,
    Claim,
    Emotion,
    PatternSignal,
    PracticeSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAtomLifecycleState {
    Candidate,
    Accepted,
    Merged,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAtomProvenanceMethod {
    Fixture,
    DeterministicRule,
    ManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAtomProvenance {
    #[serde(default)]
    pub source_trace_ids: Vec<TraceId>,
    pub method: MemoryAtomProvenanceMethod,
    pub extractor: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAtom {
    pub id: MemoryAtomId,
    pub space_id: SpaceId,
    pub namespace_id: Option<NamespaceId>,
    #[serde(default)]
    pub source_memory_ids: Vec<MemoryId>,
    pub kind: MemoryAtomKind,
    pub content: String,
    pub confidence: u8,
    pub salience: u8,
    pub state: MemoryAtomLifecycleState,
    pub provenance: MemoryAtomProvenance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
