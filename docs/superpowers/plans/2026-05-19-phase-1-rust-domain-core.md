# Phase 1 Rust Domain Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pure Rust cognitive domain core for the minimal cognitive loop before changing API or persistence.

**Architecture:** Create `src/src/domain/` as the functional core. The module must not depend on Axum, SQLx, reqwest, environment variables, Qdrant, or OpenAI. Existing API, DB, AI, and vector modules remain the imperative shell.

**Tech Stack:** Rust 1.75, `uuid`, `chrono`, `serde`, built-in unit tests, `cargo test`.

---

### Task 1: Add Cognitive Domain Types

**Files:**
- Create: `src/src/domain/mod.rs`
- Modify: `src/src/lib.rs`
- Modify: `src/src/main.rs`

- [x] **Step 1: Write failing type-level tests**

Add tests in `src/src/domain/mod.rs` proving:

```rust
let memory = Memory::new_text(space_id, actor_id, "meeting note");
assert_eq!(memory.space_id, space_id);
assert_eq!(memory.kind, MemoryKind::Text);

let lens = Lens::detective();
assert_eq!(lens.kind, LensKind::Detective);
assert!(lens.attention.signals.contains(&AttentionSignal::Anomaly));

let state = CognitiveState::new(CognitiveSpace::new(space_id, "Personal Space"));
assert_eq!(state.space.id, space_id);
assert!(state.memories.is_empty());
```

- [x] **Step 2: Run test to verify RED**

Run: `cargo test domain::tests::creates_memory_lens_and_empty_state`

Expected: FAIL because `domain` module and types do not exist.

- [x] **Step 3: Implement minimal domain types**

Create `src/src/domain/mod.rs` with:

```rust
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveSpace {
    pub id: SpaceId,
    pub name: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LensKind {
    Detective,
    Emotional,
    Systems,
    Narrative,
    Philosopher,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionSignal {
    Anomaly,
    Emotion,
    FeedbackLoop,
    IdentityContinuity,
    Meaning,
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
pub struct Relation {
    pub id: RelationId,
    pub source: CognitiveObjectRef,
    pub target: CognitiveObjectRef,
    pub kind: RelationKind,
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
```

Export it from `src/src/lib.rs` and declare it in `src/src/main.rs`.

- [x] **Step 4: Run test to verify GREEN**

Run: `cargo test domain::tests::creates_memory_lens_and_empty_state`

Expected: PASS.

### Task 2: Add Cognitive Event Evolution

**Files:**
- Modify: `src/src/domain/mod.rs`

- [x] **Step 1: Write failing event evolution tests**

Add tests proving:

```rust
let state = CognitiveState::new(space);
let memory = Memory::new_text(space_id, actor_id, "note");
let state = evolve(state, CognitiveEvent::MemoryCaptured(memory.clone()));
assert_eq!(state.memories, vec![memory]);
```

and:

```rust
let state = evolve(state, CognitiveEvent::ContradictionDetected(contradiction.clone()));
assert_eq!(state.contradictions, vec![contradiction]);
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test domain::tests::evolves_state_from_cognitive_events`

Expected: FAIL because `CognitiveEvent` and `evolve` do not exist.

- [x] **Step 3: Implement `CognitiveEvent` and `evolve`**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveEvent {
    MemoryCaptured(Memory),
    LensApplied { lens_id: LensId, memory_id: MemoryId },
    ReflectionGenerated(Reflection),
    ConceptExtracted(Concept),
    BeliefUpdated(Belief),
    RelationCreated(Relation),
    ContradictionDetected(Contradiction),
}

pub fn evolve(mut state: CognitiveState, event: CognitiveEvent) -> CognitiveState {
    match event {
        CognitiveEvent::MemoryCaptured(memory) => state.memories.push(memory),
        CognitiveEvent::ReflectionGenerated(reflection) => state.reflections.push(reflection),
        CognitiveEvent::ConceptExtracted(concept) => state.concepts.push(concept),
        CognitiveEvent::BeliefUpdated(belief) => state.beliefs.push(belief),
        CognitiveEvent::RelationCreated(relation) => state.relations.push(relation),
        CognitiveEvent::ContradictionDetected(contradiction) => {
            state.contradictions.push(contradiction);
        }
        CognitiveEvent::LensApplied { .. } => {}
    }
    state
}
```

- [x] **Step 4: Run tests to verify GREEN**

Run: `cargo test domain::tests::evolves_state_from_cognitive_events`

Expected: PASS.

### Task 3: Add Deterministic Lens Reflection

**Files:**
- Modify: `src/src/domain/mod.rs`

- [x] **Step 1: Write failing lens behavior tests**

Add tests proving the same Memory through different Lens produces different Reflection meaning:

```rust
let memory = Memory::new_text(space_id, actor_id, meeting_text);
let detective = Lens::detective();
let systems = Lens::systems();
let detective_reflection = reflect_with_lens(&memory, &detective);
let systems_reflection = reflect_with_lens(&memory, &systems);
assert_ne!(detective_reflection.meaning, systems_reflection.meaning);
```

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test domain::tests::same_memory_reflects_differently_through_different_lenses`

Expected: FAIL because `reflect_with_lens` does not exist.

- [x] **Step 3: Implement deterministic `reflect_with_lens`**

Add a pure function that returns a simple Reflection meaning based on `LensKind`. It must not call LLMs.

- [x] **Step 4: Run tests to verify GREEN**

Run: `cargo test domain::tests::same_memory_reflects_differently_through_different_lenses`

Expected: PASS.

### Task 4: Add Concept and Contradiction Helpers

**Files:**
- Modify: `src/src/domain/mod.rs`

- [x] **Step 1: Write failing abstraction tests**

Add tests proving repeated reflections produce a concept candidate and conflicting beliefs produce a contradiction.

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test domain::tests::extracts_concept_and_preserves_contradiction`

Expected: FAIL because helpers do not exist.

- [x] **Step 3: Implement pure helpers**

Add:

```rust
pub fn extract_concept(label: impl Into<String>, reflections: &[Reflection]) -> Option<Concept>
pub fn detect_contradiction(left: &Belief, right: &Belief, tension: impl Into<String>) -> Contradiction
```

- [x] **Step 4: Run tests to verify GREEN**

Run: `cargo test domain::tests::extracts_concept_and_preserves_contradiction`

Expected: PASS.

### Task 5: Full Verification

**Files:**
- Inspect all changed files.

- [x] **Step 1: Run domain tests**

Run: `cargo test domain`

Expected: PASS.

- [x] **Step 2: Run crate tests**

Run: `cargo test`

Expected: PASS.

- [x] **Step 3: Check formatting and status**

Run: `cargo fmt --check`

Expected: PASS.

Run: `git status --short`

Expected: only Phase 1 files changed.
