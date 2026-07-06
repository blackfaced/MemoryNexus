use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::memory_atom::MemoryAtom;
use super::SpaceId;

pub type CognitiveSceneId = Uuid;
pub type NamespaceId = Uuid;
pub type TraceId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveSceneType {
    Theme,
    PracticeField,
    ContradictionField,
    ProjectField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveSceneLifecycleState {
    Candidate,
    Active,
    Dormant,
    Superseded,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveSceneProvenanceMethod {
    Fixture,
    DeterministicRule,
    ManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveSceneProvenance {
    #[serde(default)]
    pub source_trace_ids: Vec<TraceId>,
    pub method: CognitiveSceneProvenanceMethod,
    pub builder: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveScene {
    pub id: CognitiveSceneId,
    pub space_id: SpaceId,
    pub namespace_id: Option<NamespaceId>,
    pub scene_type: CognitiveSceneType,
    pub title: String,
    #[serde(default)]
    pub source_atom_ids: Vec<super::memory_atom::MemoryAtomId>,
    pub summary: String,
    #[serde(default)]
    pub active_patterns: Vec<String>,
    pub state: CognitiveSceneLifecycleState,
    pub provenance: CognitiveSceneProvenance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveSceneValidationError {
    SourceAtomOutsideSpace { index: usize },
}

impl CognitiveScene {
    #[allow(clippy::too_many_arguments)]
    pub fn from_atoms(
        id: CognitiveSceneId,
        space_id: SpaceId,
        namespace_id: Option<NamespaceId>,
        scene_type: CognitiveSceneType,
        title: impl Into<String>,
        source_atoms: &[MemoryAtom],
        summary: impl Into<String>,
        active_patterns: Vec<String>,
        provenance: CognitiveSceneProvenance,
        now: DateTime<Utc>,
    ) -> Result<Self, CognitiveSceneValidationError> {
        validate_source_atoms_in_space(space_id, source_atoms)?;

        Ok(Self {
            id,
            space_id,
            namespace_id,
            scene_type,
            title: title.into(),
            source_atom_ids: source_atoms.iter().map(|atom| atom.id).collect(),
            summary: summary.into(),
            active_patterns,
            state: CognitiveSceneLifecycleState::Candidate,
            provenance,
            created_at: now,
            updated_at: now,
        })
    }
}

pub fn validate_source_atoms_in_space(
    space_id: SpaceId,
    source_atoms: &[MemoryAtom],
) -> Result<(), CognitiveSceneValidationError> {
    for (index, atom) in source_atoms.iter().enumerate() {
        if atom.space_id != space_id {
            return Err(CognitiveSceneValidationError::SourceAtomOutsideSpace { index });
        }
    }

    Ok(())
}
