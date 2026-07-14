pub mod sleep_cycle;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod cognitive_scene;
pub mod dictation;
pub mod dictation_agent_demo;
pub mod dictation_observation;
pub mod dream_candidate;
pub mod event;
pub mod evidence;
pub mod growth_model;
pub mod memory_atom;
pub mod personal_feedback;
pub mod practice_plan;
pub mod reflection;
pub mod surface;
pub mod trace;

pub type SpaceId = Uuid;
pub type ActorId = Uuid;
pub type MemoryId = Uuid;
pub type ReflectionId = Uuid;
pub type ConceptId = Uuid;
pub type BeliefId = Uuid;
pub type RelationId = Uuid;
pub type ContradictionId = Uuid;
pub type LensId = Uuid;
pub type CognitiveEventId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveSpace {
    pub id: SpaceId,
    pub name: String,
}

impl CognitiveSpace {
    pub fn new(id: SpaceId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub space_id: SpaceId,
    pub created_by: ActorId,
    pub content: String,
    pub kind: MemoryKind,
    pub captured_at: DateTime<Utc>,
}

impl Memory {
    pub fn new_text(space_id: SpaceId, created_by: ActorId, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            space_id,
            created_by,
            content: content.into(),
            kind: MemoryKind::Text,
            captured_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LensKind {
    Detective,
    Systems,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensStrategyRef {
    pub name: String,
}

impl LensStrategyRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionSignal {
    Anomaly,
    FeedbackLoop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionPolicy {
    pub signals: Vec<AttentionSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lens {
    pub id: LensId,
    pub kind: LensKind,
    pub name: String,
    pub attention: AttentionPolicy,
}

impl Lens {
    pub fn detective() -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: LensKind::Detective,
            name: "Detective Lens".to_string(),
            attention: AttentionPolicy {
                signals: vec![AttentionSignal::Anomaly],
            },
        }
    }

    pub fn systems() -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: LensKind::Systems,
            name: "Systems Lens".to_string(),
            attention: AttentionPolicy {
                signals: vec![AttentionSignal::FeedbackLoop],
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reflection {
    pub id: ReflectionId,
    pub memory_id: MemoryId,
    pub lens_id: LensId,
    pub meaning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Concept {
    pub id: ConceptId,
    pub label: String,
    pub evidence: Vec<ReflectionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Belief {
    pub id: BeliefId,
    pub claim: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveObjectRef {
    Memory(MemoryId),
    Reflection(ReflectionId),
    Concept(ConceptId),
    Belief(BeliefId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    DerivesFrom,
    Abstracts,
    Supports,
    TensionsWith,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub source: CognitiveObjectRef,
    pub target: CognitiveObjectRef,
    pub kind: RelationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    pub id: ContradictionId,
    pub left: CognitiveObjectRef,
    pub right: CognitiveObjectRef,
    pub tension: String,
    pub source_memory_ids: Vec<MemoryId>,
    pub belief_ids: Vec<BeliefId>,
    pub lens_ids: Vec<LensId>,
    pub confidence: u8,
    pub status: ContradictionStatus,
    pub resolution: Option<ContradictionResolutionMode>,
    pub updated_by_event: Option<CognitiveEventId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionStatus {
    Detected,
    Acknowledged,
    Resolved,
    AcceptedAsPlural,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionResolutionMode {
    Resolved,
    AcceptedAsPlural,
    StaleConflict,
    NeedsUserJudgment,
    DomainSpecific,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySalienceStatus {
    Active,
    Deprioritized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryDeprioritizationReason {
    Stale,
    Superseded,
    LowSignal,
    Contradicted,
    UserHidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySalience {
    pub memory_id: MemoryId,
    pub score: u8,
    pub status: MemorySalienceStatus,
    pub reason: Option<MemoryDeprioritizationReason>,
    pub updated_by_event: Option<CognitiveEventId>,
}

impl MemorySalience {
    pub fn active(memory_id: MemoryId) -> Self {
        Self {
            memory_id,
            score: 100,
            status: MemorySalienceStatus::Active,
            reason: None,
            updated_by_event: None,
        }
    }

    pub fn deprioritized(
        memory_id: MemoryId,
        reason: MemoryDeprioritizationReason,
        event_id: CognitiveEventId,
    ) -> Self {
        Self {
            memory_id,
            score: 20,
            status: MemorySalienceStatus::Deprioritized,
            reason: Some(reason),
            updated_by_event: Some(event_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveState {
    pub space: CognitiveSpace,
    pub memories: Vec<Memory>,
    pub memory_salience: Vec<MemorySalience>,
    pub reflections: Vec<Reflection>,
    pub concepts: Vec<Concept>,
    pub beliefs: Vec<Belief>,
    pub relations: Vec<Relation>,
    pub contradictions: Vec<Contradiction>,
}

impl CognitiveState {
    pub fn new(space: CognitiveSpace) -> Self {
        Self {
            space,
            memories: Vec::new(),
            memory_salience: Vec::new(),
            reflections: Vec::new(),
            concepts: Vec::new(),
            beliefs: Vec::new(),
            relations: Vec::new(),
            contradictions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveEvent {
    MemoryCaptured(Memory),
    LensApplied {
        lens_id: LensId,
        memory_id: MemoryId,
    },
    ReflectionGenerated(Reflection),
    ConceptExtracted(Concept),
    BeliefUpdated(Belief),
    RelationCreated(Relation),
    ContradictionDetected(Contradiction),
    ContradictionAcknowledged {
        contradiction_id: ContradictionId,
        event_id: CognitiveEventId,
    },
    ContradictionResolved {
        contradiction_id: ContradictionId,
        resolution: ContradictionResolutionMode,
        event_id: CognitiveEventId,
    },
    ContradictionIgnored {
        contradiction_id: ContradictionId,
        event_id: CognitiveEventId,
    },
    MemoryDeprioritized {
        memory_id: MemoryId,
        reason: MemoryDeprioritizationReason,
        event_id: CognitiveEventId,
    },
    MemoryReprioritized {
        memory_id: MemoryId,
        event_id: CognitiveEventId,
    },
}

pub fn evolve(mut state: CognitiveState, event: CognitiveEvent) -> CognitiveState {
    match event {
        CognitiveEvent::MemoryCaptured(memory) => {
            upsert_memory_salience(&mut state, MemorySalience::active(memory.id));
            state.memories.push(memory);
        }
        CognitiveEvent::LensApplied { .. } => {}
        CognitiveEvent::ReflectionGenerated(reflection) => state.reflections.push(reflection),
        CognitiveEvent::ConceptExtracted(concept) => state.concepts.push(concept),
        CognitiveEvent::BeliefUpdated(belief) => state.beliefs.push(belief),
        CognitiveEvent::RelationCreated(relation) => state.relations.push(relation),
        CognitiveEvent::ContradictionDetected(contradiction) => {
            state.contradictions.push(contradiction);
        }
        CognitiveEvent::ContradictionAcknowledged {
            contradiction_id,
            event_id,
        } => {
            update_contradiction(&mut state, contradiction_id, event_id, |contradiction| {
                contradiction.status = ContradictionStatus::Acknowledged;
            });
        }
        CognitiveEvent::ContradictionResolved {
            contradiction_id,
            resolution,
            event_id,
        } => {
            update_contradiction(&mut state, contradiction_id, event_id, |contradiction| {
                contradiction.status = match resolution {
                    ContradictionResolutionMode::AcceptedAsPlural => {
                        ContradictionStatus::AcceptedAsPlural
                    }
                    ContradictionResolutionMode::NeedsUserJudgment => {
                        ContradictionStatus::Acknowledged
                    }
                    _ => ContradictionStatus::Resolved,
                };
                contradiction.resolution = Some(resolution);
            });
        }
        CognitiveEvent::ContradictionIgnored {
            contradiction_id,
            event_id,
        } => {
            update_contradiction(&mut state, contradiction_id, event_id, |contradiction| {
                contradiction.status = ContradictionStatus::Ignored;
            });
        }
        CognitiveEvent::MemoryDeprioritized {
            memory_id,
            reason,
            event_id,
        } => {
            upsert_memory_salience(
                &mut state,
                MemorySalience::deprioritized(memory_id, reason, event_id),
            );
        }
        CognitiveEvent::MemoryReprioritized {
            memory_id,
            event_id,
        } => {
            upsert_memory_salience(
                &mut state,
                MemorySalience {
                    updated_by_event: Some(event_id),
                    ..MemorySalience::active(memory_id)
                },
            );
        }
    }

    state
}

pub fn memory_salience(state: &CognitiveState, memory_id: MemoryId) -> MemorySalience {
    state
        .memory_salience
        .iter()
        .find(|salience| salience.memory_id == memory_id)
        .cloned()
        .unwrap_or_else(|| MemorySalience::active(memory_id))
}

pub fn active_memory_ids(state: &CognitiveState) -> Vec<MemoryId> {
    state
        .memories
        .iter()
        .filter_map(|memory| {
            let salience = memory_salience(state, memory.id);
            (salience.status == MemorySalienceStatus::Active).then_some(memory.id)
        })
        .collect()
}

pub fn deprioritized_memory_ids(state: &CognitiveState) -> Vec<MemoryId> {
    state
        .memories
        .iter()
        .filter_map(|memory| {
            let salience = memory_salience(state, memory.id);
            (salience.status == MemorySalienceStatus::Deprioritized).then_some(memory.id)
        })
        .collect()
}

fn upsert_memory_salience(state: &mut CognitiveState, salience: MemorySalience) {
    if let Some(existing) = state
        .memory_salience
        .iter_mut()
        .find(|existing| existing.memory_id == salience.memory_id)
    {
        *existing = salience;
    } else {
        state.memory_salience.push(salience);
    }
}

fn update_contradiction(
    state: &mut CognitiveState,
    contradiction_id: ContradictionId,
    event_id: CognitiveEventId,
    update: impl FnOnce(&mut Contradiction),
) {
    if let Some(contradiction) = state
        .contradictions
        .iter_mut()
        .find(|contradiction| contradiction.id == contradiction_id)
    {
        update(contradiction);
        contradiction.updated_by_event = Some(event_id);
    }
}

pub fn reflect_with_lens(memory: &Memory, lens: &Lens) -> Reflection {
    let meaning = match lens.kind {
        LensKind::Detective => {
            format!(
                "Detective interpretation: look for anomaly, missing context, and hidden dynamics in '{}'.",
                memory.content
            )
        }
        LensKind::Systems => {
            format!(
                "Systems interpretation: look for feedback loops, incentives, and structural causes in '{}'.",
                memory.content
            )
        }
    };

    Reflection {
        id: Uuid::new_v4(),
        memory_id: memory.id,
        lens_id: lens.id,
        meaning,
    }
}

pub fn extract_concept(label: impl Into<String>, reflections: &[Reflection]) -> Option<Concept> {
    if reflections.len() < 2 {
        return None;
    }

    Some(Concept {
        id: Uuid::new_v4(),
        label: label.into(),
        evidence: reflections.iter().map(|reflection| reflection.id).collect(),
    })
}

pub fn detect_contradiction(
    left: &Belief,
    right: &Belief,
    tension: impl Into<String>,
) -> Contradiction {
    detect_contradiction_with_sources(
        CognitiveObjectRef::Belief(left.id),
        CognitiveObjectRef::Belief(right.id),
        tension,
        vec![],
        vec![left.id, right.id],
        vec![],
        left.confidence.min(right.confidence),
    )
}

pub fn detect_contradiction_with_sources(
    left: CognitiveObjectRef,
    right: CognitiveObjectRef,
    tension: impl Into<String>,
    source_memory_ids: Vec<MemoryId>,
    belief_ids: Vec<BeliefId>,
    lens_ids: Vec<LensId>,
    confidence: u8,
) -> Contradiction {
    Contradiction {
        id: Uuid::new_v4(),
        left,
        right,
        tension: tension.into(),
        source_memory_ids,
        belief_ids,
        lens_ids,
        confidence,
        status: ContradictionStatus::Detected,
        resolution: None,
        updated_by_event: None,
    }
}

pub fn unresolved_contradictions(state: &CognitiveState) -> Vec<Contradiction> {
    state
        .contradictions
        .iter()
        .filter(|contradiction| {
            matches!(
                contradiction.status,
                ContradictionStatus::Detected | ContradictionStatus::Acknowledged
            )
        })
        .cloned()
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileTarget {
    LlmContext,
    Project,
    Learning,
    Family,
    Risk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveProfile {
    pub space_id: SpaceId,
    pub lens_id: Option<LensId>,
    pub target: ProfileTarget,
    pub version: u64,
    pub stable_beliefs: Vec<Belief>,
    pub active_concepts: Vec<Concept>,
    pub current_goals: Vec<String>,
    pub unresolved_contradictions: Vec<Contradiction>,
    pub summary: String,
    pub source_memory_ids: Vec<MemoryId>,
    pub source_event_ids: Vec<CognitiveEventId>,
}

pub fn project_profile(
    state: &CognitiveState,
    target: ProfileTarget,
    lens_id: Option<LensId>,
    source_event_ids: Vec<CognitiveEventId>,
) -> CognitiveProfile {
    let stable_beliefs = state
        .beliefs
        .iter()
        .filter(|belief| belief.confidence >= 60)
        .cloned()
        .collect::<Vec<_>>();
    let active_concepts = state.concepts.iter().take(8).cloned().collect::<Vec<_>>();
    let unresolved_contradictions = unresolved_contradictions(state);
    let source_memory_ids = active_memory_ids(state);
    let current_goals = infer_current_goals(&target, &stable_beliefs, &active_concepts);

    CognitiveProfile {
        space_id: state.space.id,
        lens_id,
        target,
        version: 1,
        stable_beliefs,
        active_concepts,
        current_goals,
        unresolved_contradictions,
        summary: profile_summary(state),
        source_memory_ids,
        source_event_ids,
    }
}

fn infer_current_goals(
    target: &ProfileTarget,
    stable_beliefs: &[Belief],
    active_concepts: &[Concept],
) -> Vec<String> {
    match target {
        ProfileTarget::Project if !stable_beliefs.is_empty() || !active_concepts.is_empty() => {
            vec!["Use the profile as compact project context for the next Lens run.".to_string()]
        }
        ProfileTarget::Learning if !active_concepts.is_empty() => {
            vec!["Turn active learning concepts into the next smallest practice step.".to_string()]
        }
        ProfileTarget::Risk if !stable_beliefs.is_empty() => {
            vec!["Review stable beliefs for unsupported risk assumptions.".to_string()]
        }
        _ => vec![],
    }
}

fn profile_summary(state: &CognitiveState) -> String {
    let active_memory_count = active_memory_ids(state).len();
    let deprioritized_memory_count = deprioritized_memory_ids(state).len();

    format!(
        "Cognitive profile for '{}' with {} active memories, {} deprioritized memories, {} concepts, {} stable beliefs, and {} unresolved contradictions.",
        state.space.name,
        active_memory_count,
        deprioritized_memory_count,
        state.concepts.len(),
        state
            .beliefs
            .iter()
            .filter(|belief| belief.confidence >= 60)
            .count(),
        unresolved_contradictions(state).len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn creates_memory_lens_and_empty_state() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let memory = Memory::new_text(space_id, actor_id, "meeting note");
        assert_eq!(memory.space_id, space_id);
        assert_eq!(memory.created_by, actor_id);
        assert_eq!(memory.kind, MemoryKind::Text);
        assert_eq!(memory.content, "meeting note");

        let lens = Lens::detective();
        assert_eq!(lens.kind, LensKind::Detective);
        assert!(lens.attention.signals.contains(&AttentionSignal::Anomaly));

        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Personal Space"));
        assert_eq!(state.space.id, space_id);
        assert!(state.memories.is_empty());
        assert!(state.memory_salience.is_empty());
        assert!(state.reflections.is_empty());
        assert!(state.concepts.is_empty());
        assert!(state.beliefs.is_empty());
        assert!(state.relations.is_empty());
        assert!(state.contradictions.is_empty());
    }

    #[test]
    fn evolves_state_from_cognitive_events() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let space = CognitiveSpace::new(space_id, "Personal Space");
        let state = CognitiveState::new(space);
        let memory = Memory::new_text(space_id, actor_id, "I noticed a recurring pattern.");

        let state = evolve(state, CognitiveEvent::MemoryCaptured(memory.clone()));
        assert_eq!(state.memories, vec![memory]);
        assert_eq!(state.memory_salience.len(), 1);
        assert_eq!(
            state.memory_salience[0].status,
            MemorySalienceStatus::Active
        );
        assert!(state.contradictions.is_empty());

        let contradiction = detect_contradiction_with_sources(
            CognitiveObjectRef::Memory(state.memories[0].id),
            CognitiveObjectRef::Memory(state.memories[0].id),
            "The same event can feel both safe and risky.",
            vec![state.memories[0].id],
            vec![],
            vec![],
            70,
        );

        let state = evolve(
            state,
            CognitiveEvent::ContradictionDetected(contradiction.clone()),
        );
        assert_eq!(state.contradictions, vec![contradiction]);
    }

    #[test]
    fn same_memory_reflects_differently_through_different_lenses() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let memory = Memory::new_text(
            space_id,
            actor_id,
            "I raised an idea in a meeting. Nobody responded until a colleague repeated it.",
        );

        let detective = Lens::detective();
        let systems = Lens::systems();

        let detective_reflection = reflect_with_lens(&memory, &detective);
        let systems_reflection = reflect_with_lens(&memory, &systems);

        assert_eq!(detective_reflection.memory_id, memory.id);
        assert_eq!(systems_reflection.memory_id, memory.id);
        assert_eq!(detective_reflection.lens_id, detective.id);
        assert_eq!(systems_reflection.lens_id, systems.id);
        assert_ne!(detective_reflection.meaning, systems_reflection.meaning);
        assert!(detective_reflection.meaning.contains("anomaly"));
        assert!(systems_reflection.meaning.contains("feedback"));
    }

    #[test]
    fn extracts_concept_and_preserves_contradiction() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let memory = Memory::new_text(
            space_id,
            actor_id,
            "I often hesitate to speak even when I want to be understood.",
        );
        let detective_reflection = reflect_with_lens(&memory, &Lens::detective());
        let systems_reflection = reflect_with_lens(&memory, &Lens::systems());
        let reflections = vec![detective_reflection.clone(), systems_reflection.clone()];

        let concept = extract_concept("voice hesitation", &reflections)
            .expect("two reflections should create a concept candidate");

        assert_eq!(concept.label, "voice hesitation");
        assert_eq!(
            concept.evidence,
            vec![detective_reflection.id, systems_reflection.id]
        );

        let wanting_connection = Belief {
            id: Uuid::new_v4(),
            claim: "I want to be understood.".to_string(),
            confidence: 80,
        };
        let fearing_exposure = Belief {
            id: Uuid::new_v4(),
            claim: "I fear exposing what I really think.".to_string(),
            confidence: 70,
        };

        let contradiction = detect_contradiction(
            &wanting_connection,
            &fearing_exposure,
            "desire for connection conflicts with fear of exposure",
        );

        assert_eq!(
            contradiction.left,
            CognitiveObjectRef::Belief(wanting_connection.id)
        );
        assert_eq!(
            contradiction.right,
            CognitiveObjectRef::Belief(fearing_exposure.id)
        );
        assert!(contradiction.tension.contains("fear of exposure"));
        assert_eq!(contradiction.belief_ids.len(), 2);
        assert_eq!(contradiction.confidence, 70);
        assert_eq!(contradiction.status, ContradictionStatus::Detected);
    }

    #[test]
    fn profile_projects_state_without_owning_memories() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let lens_id = Uuid::new_v4();
        let memory = Memory::new_text(space_id, actor_id, "Ship a small CLI MVP before UI work.");
        let reflection = Reflection {
            id: Uuid::new_v4(),
            memory_id: memory.id,
            lens_id,
            meaning: "Project should favor CLI feedback loops.".to_string(),
        };
        let concept = Concept {
            id: Uuid::new_v4(),
            label: "CLI-first validation".to_string(),
            evidence: vec![reflection.id],
        };
        let belief = Belief {
            id: Uuid::new_v4(),
            claim: "Validate the cognitive loop through CLI before UI.".to_string(),
            confidence: 82,
        };
        let event_id = Uuid::new_v4();
        let state = CognitiveState {
            space: CognitiveSpace::new(space_id, "Project Space"),
            memories: vec![memory.clone()],
            memory_salience: vec![MemorySalience::active(memory.id)],
            reflections: vec![reflection],
            concepts: vec![concept.clone()],
            beliefs: vec![belief.clone()],
            relations: vec![],
            contradictions: vec![],
        };

        let profile = project_profile(
            &state,
            ProfileTarget::Project,
            Some(lens_id),
            vec![event_id],
        );

        assert_eq!(profile.space_id, space_id);
        assert_eq!(profile.lens_id, Some(lens_id));
        assert_eq!(profile.stable_beliefs, vec![belief]);
        assert_eq!(profile.active_concepts, vec![concept]);
        assert_eq!(profile.source_memory_ids, vec![memory.id]);
        assert_eq!(profile.source_event_ids, vec![event_id]);
        assert!(profile.summary.contains("Project Space"));
    }

    #[test]
    fn profile_keeps_only_stable_beliefs_and_unresolved_contradictions() {
        let space_id = Uuid::new_v4();
        let high_confidence = Belief {
            id: Uuid::new_v4(),
            claim: "Use Cognitive Space as the ownership boundary.".to_string(),
            confidence: 80,
        };
        let low_confidence = Belief {
            id: Uuid::new_v4(),
            claim: "Maybe memory belongs to agents.".to_string(),
            confidence: 30,
        };
        let contradiction = detect_contradiction(
            &high_confidence,
            &low_confidence,
            "ownership boundary remains unresolved",
        );
        let state = CognitiveState {
            space: CognitiveSpace::new(space_id, "Architecture Space"),
            memories: vec![],
            memory_salience: vec![],
            reflections: vec![],
            concepts: vec![],
            beliefs: vec![low_confidence, high_confidence.clone()],
            relations: vec![],
            contradictions: vec![contradiction.clone()],
        };

        let profile = project_profile(&state, ProfileTarget::LlmContext, None, vec![]);

        assert_eq!(profile.stable_beliefs, vec![high_confidence]);
        assert_eq!(profile.unresolved_contradictions, vec![contradiction]);
        assert!(profile.current_goals.is_empty());
    }

    #[test]
    fn contradiction_lifecycle_tracks_acknowledge_resolve_and_unresolved_projection() {
        let space_id = Uuid::new_v4();
        let belief_a = Belief {
            id: Uuid::new_v4(),
            claim: "Ship fast.".to_string(),
            confidence: 90,
        };
        let belief_b = Belief {
            id: Uuid::new_v4(),
            claim: "Avoid risky releases.".to_string(),
            confidence: 85,
        };
        let contradiction =
            detect_contradiction(&belief_a, &belief_b, "speed conflicts with release risk");
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Project Space"));
        let state = evolve(
            state,
            CognitiveEvent::ContradictionDetected(contradiction.clone()),
        );

        assert_eq!(
            unresolved_contradictions(&state),
            vec![contradiction.clone()]
        );

        let acknowledge_event = Uuid::new_v4();
        let state = evolve(
            state,
            CognitiveEvent::ContradictionAcknowledged {
                contradiction_id: contradiction.id,
                event_id: acknowledge_event,
            },
        );
        assert_eq!(
            state.contradictions[0].status,
            ContradictionStatus::Acknowledged
        );
        assert_eq!(
            state.contradictions[0].updated_by_event,
            Some(acknowledge_event)
        );
        assert_eq!(unresolved_contradictions(&state).len(), 1);

        let resolve_event = Uuid::new_v4();
        let state = evolve(
            state,
            CognitiveEvent::ContradictionResolved {
                contradiction_id: contradiction.id,
                resolution: ContradictionResolutionMode::Resolved,
                event_id: resolve_event,
            },
        );
        let profile = project_profile(&state, ProfileTarget::Risk, None, vec![resolve_event]);

        assert_eq!(
            state.contradictions[0].status,
            ContradictionStatus::Resolved
        );
        assert_eq!(
            state.contradictions[0].resolution,
            Some(ContradictionResolutionMode::Resolved)
        );
        assert!(profile.unresolved_contradictions.is_empty());
    }

    #[test]
    fn contradiction_can_be_accepted_as_plural_across_lenses() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let emotional_lens_id = Uuid::new_v4();
        let systems_lens_id = Uuid::new_v4();
        let memory = Memory::new_text(
            space_id,
            actor_id,
            "The same decision felt kind and costly.",
        );
        let belief_a = Belief {
            id: Uuid::new_v4(),
            claim: "The decision protected relationship safety.".to_string(),
            confidence: 80,
        };
        let belief_b = Belief {
            id: Uuid::new_v4(),
            claim: "The decision increased system cost.".to_string(),
            confidence: 78,
        };
        let contradiction = detect_contradiction_with_sources(
            CognitiveObjectRef::Belief(belief_a.id),
            CognitiveObjectRef::Belief(belief_b.id),
            "relationship safety and system cost can both be true under different lenses",
            vec![memory.id],
            vec![belief_a.id, belief_b.id],
            vec![emotional_lens_id, systems_lens_id],
            78,
        );
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Plural Truth Space"));
        let state = evolve(
            state,
            CognitiveEvent::ContradictionDetected(contradiction.clone()),
        );
        let event_id = Uuid::new_v4();

        let state = evolve(
            state,
            CognitiveEvent::ContradictionResolved {
                contradiction_id: contradiction.id,
                resolution: ContradictionResolutionMode::AcceptedAsPlural,
                event_id,
            },
        );
        let updated = &state.contradictions[0];

        assert_eq!(updated.status, ContradictionStatus::AcceptedAsPlural);
        assert_eq!(
            updated.resolution,
            Some(ContradictionResolutionMode::AcceptedAsPlural)
        );
        assert_eq!(updated.source_memory_ids, vec![memory.id]);
        assert_eq!(updated.belief_ids, vec![belief_a.id, belief_b.id]);
        assert_eq!(updated.lens_ids, vec![emotional_lens_id, systems_lens_id]);
        assert!(unresolved_contradictions(&state).is_empty());
    }

    #[test]
    fn contradiction_can_remain_acknowledged_when_user_judgment_is_needed() {
        let space_id = Uuid::new_v4();
        let belief_a = Belief {
            id: Uuid::new_v4(),
            claim: "Prefer privacy.".to_string(),
            confidence: 75,
        };
        let belief_b = Belief {
            id: Uuid::new_v4(),
            claim: "Prefer shared context.".to_string(),
            confidence: 75,
        };
        let contradiction = detect_contradiction(&belief_a, &belief_b, "privacy vs shared memory");
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Family Space"));
        let state = evolve(
            state,
            CognitiveEvent::ContradictionDetected(contradiction.clone()),
        );

        let state = evolve(
            state,
            CognitiveEvent::ContradictionResolved {
                contradiction_id: contradiction.id,
                resolution: ContradictionResolutionMode::NeedsUserJudgment,
                event_id: Uuid::new_v4(),
            },
        );

        assert_eq!(
            state.contradictions[0].status,
            ContradictionStatus::Acknowledged
        );
        assert_eq!(
            state.contradictions[0].resolution,
            Some(ContradictionResolutionMode::NeedsUserJudgment)
        );
        assert_eq!(unresolved_contradictions(&state).len(), 1);
    }

    #[test]
    fn memory_deprioritization_keeps_memory_but_removes_from_default_profile_sources() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let memory = Memory::new_text(space_id, actor_id, "Temporary scratch note.");
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Personal Space"));
        let state = evolve(state, CognitiveEvent::MemoryCaptured(memory.clone()));
        let event_id = Uuid::new_v4();

        let state = evolve(
            state,
            CognitiveEvent::MemoryDeprioritized {
                memory_id: memory.id,
                reason: MemoryDeprioritizationReason::Superseded,
                event_id,
            },
        );
        let profile = project_profile(&state, ProfileTarget::LlmContext, None, vec![event_id]);

        assert_eq!(state.memories, vec![memory.clone()]);
        assert_eq!(deprioritized_memory_ids(&state), vec![memory.id]);
        assert!(profile.source_memory_ids.is_empty());
    }

    #[test]
    fn memory_reprioritization_restores_default_profile_recall() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let memory = Memory::new_text(space_id, actor_id, "A useful project direction note.");
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Project Space"));
        let state = evolve(state, CognitiveEvent::MemoryCaptured(memory.clone()));
        let state = evolve(
            state,
            CognitiveEvent::MemoryDeprioritized {
                memory_id: memory.id,
                reason: MemoryDeprioritizationReason::Stale,
                event_id: Uuid::new_v4(),
            },
        );
        let event_id = Uuid::new_v4();

        let state = evolve(
            state,
            CognitiveEvent::MemoryReprioritized {
                memory_id: memory.id,
                event_id,
            },
        );
        let profile = project_profile(&state, ProfileTarget::Project, None, vec![event_id]);

        assert_eq!(active_memory_ids(&state), vec![memory.id]);
        assert_eq!(profile.source_memory_ids, vec![memory.id]);
    }

    #[test]
    fn deprioritization_reason_is_traceable() {
        let space_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let memory = Memory::new_text(space_id, actor_id, "Contradicted assumption.");
        let state = CognitiveState::new(CognitiveSpace::new(space_id, "Risk Space"));
        let state = evolve(state, CognitiveEvent::MemoryCaptured(memory.clone()));
        let event_id = Uuid::new_v4();

        let state = evolve(
            state,
            CognitiveEvent::MemoryDeprioritized {
                memory_id: memory.id,
                reason: MemoryDeprioritizationReason::Contradicted,
                event_id,
            },
        );
        let salience = memory_salience(&state, memory.id);

        assert_eq!(salience.status, MemorySalienceStatus::Deprioritized);
        assert_eq!(
            salience.reason,
            Some(MemoryDeprioritizationReason::Contradicted)
        );
        assert_eq!(salience.updated_by_event, Some(event_id));
    }
}
