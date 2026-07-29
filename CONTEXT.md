# MemoryNexus

MemoryNexus is a user-owned long-term feedback engine that turns source-linked
evidence from independent products into longitudinal observations and bounded
next actions.

## Ownership and scope

**CognitiveSpace**:
The ownership, membership, and data-subject boundary for a person's
longitudinal evidence and feedback.
_Avoid_: user namespace, provider space, shared profile

**Managed CognitiveSpace**:
A private CognitiveSpace whose data subject does not currently administer its
membership. A designated member manages it under a disclosed, bounded policy
with correction, deletion, and synchronization controls.
_Avoid_: parent profile, family-wide space, silent monitoring

**Namespace**:
A domain partition inside one CognitiveSpace. It organizes evidence but does
not grant access, identify a person, or represent a provider.
_Avoid_: permission boundary, person identifier, provider namespace

## External evidence

**Source Record**:
A durable, provider-owned record of an action, expression, or outcome in the
source product's native language.
_Avoid_: MemoryNexus event, external memory, universal event

**Source Identity**:
The stable identity of a Source Record across retries, migration, revision,
and withdrawal. It distinguishes the source product and installation as well
as the provider-native record.
_Avoid_: mutable hostname, file path, person name, generated MemoryNexus ID

**Source Revision**:
A source-authoritative replacement for the current content of an existing
Source Record. It supersedes the earlier revision without becoming unrelated
evidence.
_Avoid_: duplicate event, edited memory, new observation

**Source Tombstone**:
A source-authoritative withdrawal of a Source Record's content. It retains only
the minimum identity needed to prevent re-import while invalidating dependent
summaries and interpretations.
_Avoid_: soft-hidden content, archived evidence, empty record

**Reference Adapter**:
A replaceable boundary that reads Source Records and maps them into
provider-neutral evidence without making the source product depend on
MemoryNexus.
_Avoid_: embedded integration, provider branch, universal connector

**Normalized Outcome**:
Provider-neutral, source-attributed evidence derived from one or more Source
Records and eligible for submission to MemoryNexus.
_Avoid_: raw event, copied database row, authoritative growth state

**Evidence Trust**:
The declared authority of evidence for longitudinal interpretation. Contract-
trusted outcomes may participate automatically, while model-derived summaries
remain observational until an owner confirms or corrects them.
_Avoid_: model confidence, truth score, provider reputation

## Learning journey

**Self-Directed Learning**:
Learning selected and directed by the data subject for their own development.
It is the domain represented by the `learning.self-directed` Namespace.
_Avoid_: DeepTutor learning, adult namespace, personal provider data

**Foundational Learning**:
Guided learning evidence, outcomes, and journey observations for a learner in a
Managed CognitiveSpace. It is the domain represented by the
`learning.foundation` Namespace.
_Avoid_: Study Buddy data, child profile, tutoring-provider namespace

**Learning Attempt**:
A source-linked learner response to a bounded task with an observable or
evaluable outcome.
_Avoid_: complete conversation, model reasoning, generic activity

**Learning Session**:
A bounded period of related learning activity summarized by its participation
and outcome measures. It does not contain a raw interaction transcript.
_Avoid_: chat log, process run, arbitrary time bucket

**Learner Journey Summary**:
A bounded, source-linked summary of a learner's explicit expressions and
observable changes during a defined period. It is not a psychological
diagnosis, stable personality judgment, or inference about hidden motives.
_Avoid_: psychological profile, personality assessment, hidden mental state

**Summary Window**:
A defined source and time interval covered by exactly one current Learner
Journey Summary. A later revision replaces the current summary for the same
window rather than creating another independent observation.
_Avoid_: cron run, daily batch, duplicate summary
