use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, NaiveTime};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::growth_model::EvidenceId;

pub const BASELINE_WINDOW_KIND: &str = "latest_evidence_anchored_rolling_local_dates";
pub const BASELINE_WINDOW_DURATION_DAYS: i64 = 14;
pub const BASELINE_THRESHOLD: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepObservationEvidenceRecord {
    pub memory_id: Uuid,
    pub local_date: NaiveDate,
    pub sleep_duration_minutes: i32,
    pub daytime_energy: i32,
    pub sleep_timing_present: bool,
    pub screen_minutes_before_sleep: Option<i32>,
    pub input_source: SleepObservationInputSource,
    pub confirmation_method: SleepObservationConfirmationMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepObservationInputSource {
    Typed,
    AgentOcr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepObservationConfirmationMethod {
    ExplicitAcceptance,
    ExplicitCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalFeedbackObservationStatus {
    NeedsMoreEvidence,
    BaselineReady,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PersonalFeedbackObservationSummary {
    pub status: PersonalFeedbackObservationStatus,
    pub window: PersonalFeedbackObservationWindow,
    pub valid_record_count: usize,
    pub threshold: usize,
    pub remaining_record_count: usize,
    pub supporting_evidence_ids: Vec<EvidenceId>,
    pub observed: Option<SleepBaselineObserved>,
    pub hypotheses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonalFeedbackObservationWindow {
    pub kind: &'static str,
    pub duration_days: i64,
    pub inclusive: bool,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SleepBaselineObserved {
    pub sleep_duration_minutes: SleepDurationObserved,
    pub daytime_energy: DaytimeEnergyObserved,
    pub sleep_timing: CoverageObserved,
    pub input_sources: BTreeMap<String, usize>,
    pub confirmations: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SleepDurationObserved {
    pub coverage_count: usize,
    pub min: i32,
    pub max: i32,
    pub median: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DaytimeEnergyObserved {
    pub coverage_count: usize,
    pub distribution: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageObserved {
    pub coverage_count: usize,
}

impl SleepObservationEvidenceRecord {
    pub fn from_persistence_metadata(memory_id: Uuid, metadata: &Value) -> Option<Self> {
        let record = metadata.pointer("/capture/personal_feedback")?;
        if record.get("record_type")?.as_str()? != "sleep_energy_check_in" {
            return None;
        }
        let local_date = parse_date(record.get("local_date")?.as_str()?)?;
        let sleep_duration_minutes =
            i32::try_from(record.get("sleep_duration_minutes")?.as_i64()?).ok()?;
        let daytime_energy = i32::try_from(record.get("daytime_energy")?.as_i64()?).ok()?;
        if !(60..=960).contains(&sleep_duration_minutes) || !(1..=5).contains(&daytime_energy) {
            return None;
        }
        let input_source = match record.get("input_source")?.as_str()? {
            "typed" => SleepObservationInputSource::Typed,
            "agent_ocr" => SleepObservationInputSource::AgentOcr,
            _ => return None,
        };
        let confirmation = record.get("input_confirmation")?;
        if confirmation.get("status")?.as_str()? != "confirmed" {
            return None;
        }
        let confirmation_method = match confirmation.get("method")?.as_str()? {
            "explicit_acceptance" => SleepObservationConfirmationMethod::ExplicitAcceptance,
            "explicit_correction" => SleepObservationConfirmationMethod::ExplicitCorrection,
            _ => return None,
        };
        match (confirmation_method, record.get("corrects_record_id")) {
            (SleepObservationConfirmationMethod::ExplicitAcceptance, None) => {}
            (SleepObservationConfirmationMethod::ExplicitCorrection, Some(value)) => {
                Uuid::parse_str(value.as_str()?).ok()?;
            }
            _ => return None,
        }
        let start = optional_time(record, "sleep_start_local_time")?;
        let end = optional_time(record, "sleep_end_local_time")?;
        optional_boolean(record, "caffeine_within_six_hours_of_sleep")?;
        let screen_minutes_before_sleep = optional_screen_minutes(record)?;
        if let (Some(start), Some(end)) = (start, end) {
            let circular_minutes = (end - start).num_minutes().rem_euclid(24 * 60);
            if (circular_minutes - i64::from(sleep_duration_minutes)).abs() > 60 {
                return None;
            }
        }
        Some(Self {
            memory_id,
            local_date,
            sleep_duration_minutes,
            daytime_energy,
            sleep_timing_present: start.is_some() && end.is_some(),
            screen_minutes_before_sleep,
            input_source,
            confirmation_method,
        })
    }
}

pub fn build_personal_feedback_observation_summary(
    evidence_records: Vec<SleepObservationEvidenceRecord>,
) -> PersonalFeedbackObservationSummary {
    let mut by_date = BTreeMap::<NaiveDate, Vec<SleepObservationEvidenceRecord>>::new();
    for record in evidence_records {
        by_date.entry(record.local_date).or_default().push(record);
    }
    // A duplicate current record is invalid evidence. Exclude that whole date rather
    // than choosing an arbitrary row.
    let eligible = by_date
        .into_values()
        .filter_map(|records| {
            (records.len() == 1).then(|| records.into_iter().next().expect("one record"))
        })
        .collect::<Vec<_>>();
    let window = eligible.last().map(|record| {
        let end = record.local_date;
        (end - Duration::days(BASELINE_WINDOW_DURATION_DAYS - 1), end)
    });
    let selected = match window {
        Some((start, end)) => eligible
            .into_iter()
            .filter(|record| record.local_date >= start && record.local_date <= end)
            .collect::<Vec<_>>(),
        None => Vec::new(),
    };
    let supporting_evidence_ids = selected
        .iter()
        .map(|record| EvidenceId::Memory(record.memory_id))
        .collect::<Vec<_>>();
    let valid_record_count = selected.len();
    let window = PersonalFeedbackObservationWindow {
        kind: BASELINE_WINDOW_KIND,
        duration_days: BASELINE_WINDOW_DURATION_DAYS,
        inclusive: true,
        start_local_date: window.map(|(start, _)| start.to_string()),
        end_local_date: window.map(|(_, end)| end.to_string()),
    };
    if valid_record_count < BASELINE_THRESHOLD {
        return PersonalFeedbackObservationSummary {
            status: PersonalFeedbackObservationStatus::NeedsMoreEvidence,
            window,
            valid_record_count,
            threshold: BASELINE_THRESHOLD,
            remaining_record_count: BASELINE_THRESHOLD - valid_record_count,
            supporting_evidence_ids,
            observed: None,
            hypotheses: Vec::new(),
        };
    }
    let mut durations = selected
        .iter()
        .map(|record| record.sleep_duration_minutes)
        .collect::<Vec<_>>();
    durations.sort_unstable();
    let middle = durations.len() / 2;
    let median = if durations.len() % 2 == 0 {
        (f64::from(durations[middle - 1]) + f64::from(durations[middle])) / 2.0
    } else {
        f64::from(durations[middle])
    };
    let mut energy_distribution = BTreeMap::new();
    let mut input_sources = BTreeMap::new();
    let mut confirmations = BTreeMap::new();
    for record in &selected {
        *energy_distribution
            .entry(record.daytime_energy.to_string())
            .or_insert(0) += 1;
        *input_sources
            .entry(match record.input_source {
                SleepObservationInputSource::Typed => "typed".to_string(),
                SleepObservationInputSource::AgentOcr => "agent_ocr".to_string(),
            })
            .or_insert(0) += 1;
        *confirmations
            .entry(match record.confirmation_method {
                SleepObservationConfirmationMethod::ExplicitAcceptance => {
                    "explicit_acceptance".to_string()
                }
                SleepObservationConfirmationMethod::ExplicitCorrection => {
                    "explicit_correction".to_string()
                }
            })
            .or_insert(0) += 1;
    }
    let timing_coverage = selected
        .iter()
        .filter(|record| record.sleep_timing_present)
        .count();
    PersonalFeedbackObservationSummary {
        status: PersonalFeedbackObservationStatus::BaselineReady,
        window,
        valid_record_count,
        threshold: BASELINE_THRESHOLD,
        remaining_record_count: 0,
        supporting_evidence_ids,
        observed: Some(SleepBaselineObserved {
            sleep_duration_minutes: SleepDurationObserved {
                coverage_count: durations.len(),
                min: durations[0],
                max: *durations.last().expect("non-empty"),
                median,
            },
            daytime_energy: DaytimeEnergyObserved {
                coverage_count: valid_record_count,
                distribution: energy_distribution,
            },
            sleep_timing: CoverageObserved {
                coverage_count: timing_coverage,
            },
            input_sources,
            confirmations,
        }),
        hypotheses: Vec::new(),
    }
}

fn parse_time(value: &str) -> Result<NaiveTime, ()> {
    let time = NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| ())?;
    (time.format("%H:%M").to_string() == value)
        .then_some(time)
        .ok_or(())
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    (date.format("%Y-%m-%d").to_string() == value).then_some(date)
}

fn optional_time(record: &Value, field: &str) -> Option<Option<NaiveTime>> {
    match record.get(field) {
        None => Some(None),
        Some(value) => Some(Some(parse_time(value.as_str()?).ok()?)),
    }
}

fn optional_boolean(record: &Value, field: &str) -> Option<Option<bool>> {
    match record.get(field) {
        None => Some(None),
        Some(value) => Some(Some(value.as_bool()?)),
    }
}

fn optional_screen_minutes(record: &Value) -> Option<Option<i32>> {
    match record.get("screen_minutes_in_final_hour") {
        None => Some(None),
        Some(value) => {
            let minutes = i32::try_from(value.as_i64()?).ok()?;
            (0..=60).contains(&minutes).then_some(Some(minutes))
        }
    }
}
