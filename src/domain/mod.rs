use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveState {
    pub space: CognitiveSpace,
    pub memories: Vec<Memory>,
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
}

pub fn evolve(mut state: CognitiveState, event: CognitiveEvent) -> CognitiveState {
    match event {
        CognitiveEvent::MemoryCaptured(memory) => state.memories.push(memory),
        CognitiveEvent::LensApplied { .. } => {}
        CognitiveEvent::ReflectionGenerated(reflection) => state.reflections.push(reflection),
        CognitiveEvent::ConceptExtracted(concept) => state.concepts.push(concept),
        CognitiveEvent::BeliefUpdated(belief) => state.beliefs.push(belief),
        CognitiveEvent::RelationCreated(relation) => state.relations.push(relation),
        CognitiveEvent::ContradictionDetected(contradiction) => {
            state.contradictions.push(contradiction);
        }
    }

    state
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
    Contradiction {
        id: Uuid::new_v4(),
        left: CognitiveObjectRef::Belief(left.id),
        right: CognitiveObjectRef::Belief(right.id),
        tension: tension.into(),
    }
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
    let unresolved_contradictions = state.contradictions.clone();
    let source_memory_ids = state
        .memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
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
    format!(
        "Cognitive profile for '{}' with {} memories, {} concepts, {} stable beliefs, and {} unresolved contradictions.",
        state.space.name,
        state.memories.len(),
        state.concepts.len(),
        state
            .beliefs
            .iter()
            .filter(|belief| belief.confidence >= 60)
            .count(),
        state.contradictions.len()
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
        assert!(state.contradictions.is_empty());

        let contradiction = Contradiction {
            id: Uuid::new_v4(),
            left: CognitiveObjectRef::Memory(state.memories[0].id),
            right: CognitiveObjectRef::Memory(state.memories[0].id),
            tension: "The same event can feel both safe and risky.".to_string(),
        };

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
}
