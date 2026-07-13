use std::fmt;

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::evidence::{
    validate_evidence_request, InputConfirmation, InputConfirmationMethod,
};

pub const PERSONAL_HEALTH_SLEEP_NAMESPACE: &str = "personal.health.sleep";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SleepEnergyInputSource {
    Typed,
    AgentOcr,
}

impl SleepEnergyInputSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Typed => "typed",
            Self::AgentOcr => "agent_ocr",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SleepEnergyCheckInInput {
    pub local_date: String,
    pub sleep_duration_minutes: i32,
    pub sleep_start_local_time: Option<String>,
    pub sleep_end_local_time: Option<String>,
    pub daytime_energy: i32,
    pub caffeine_within_six_hours_of_sleep: Option<bool>,
    pub screen_minutes_in_final_hour: Option<i32>,
    pub input_source: SleepEnergyInputSource,
    pub input_confirmation: InputConfirmation,
    pub corrects_record_id: Option<Uuid>,
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ConfirmedSleepEnergyCheckIn {
    pub local_date: NaiveDate,
    pub sleep_duration_minutes: i32,
    pub sleep_start_local_time: Option<NaiveTime>,
    pub sleep_end_local_time: Option<NaiveTime>,
    pub daytime_energy: i32,
    pub caffeine_within_six_hours_of_sleep: Option<bool>,
    pub screen_minutes_in_final_hour: Option<i32>,
    pub input_source: SleepEnergyInputSource,
    pub input_confirmation: InputConfirmation,
    pub corrects_record_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepEnergyValidationError(&'static str);

impl fmt::Display for SleepEnergyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SleepEnergyValidationError {}

impl SleepEnergyCheckInInput {
    pub fn confirm(self) -> Result<ConfirmedSleepEnergyCheckIn, SleepEnergyValidationError> {
        let local_date = parse_date(&self.local_date)?;
        if !(60..=960).contains(&self.sleep_duration_minutes) {
            return Err(SleepEnergyValidationError(
                "sleep_duration_minutes must be an integer between 60 and 960",
            ));
        }
        if !(1..=5).contains(&self.daytime_energy) {
            return Err(SleepEnergyValidationError(
                "daytime_energy must be an integer between 1 and 5",
            ));
        }
        if self
            .screen_minutes_in_final_hour
            .is_some_and(|minutes| !(0..=60).contains(&minutes))
        {
            return Err(SleepEnergyValidationError(
                "screen_minutes_in_final_hour must be an integer between 0 and 60",
            ));
        }

        let sleep_start_local_time = self
            .sleep_start_local_time
            .as_deref()
            .map(parse_time)
            .transpose()?;
        let sleep_end_local_time = self
            .sleep_end_local_time
            .as_deref()
            .map(parse_time)
            .transpose()?;
        if let (Some(start), Some(end)) = (sleep_start_local_time, sleep_end_local_time) {
            let circular_minutes = (end - start).num_minutes().rem_euclid(24 * 60);
            if (circular_minutes - i64::from(self.sleep_duration_minutes)).abs() > 60 {
                return Err(SleepEnergyValidationError(
                    "sleep interval must agree with sleep_duration_minutes within 60 minutes",
                ));
            }
        }

        validate_evidence_request(
            Some(self.input_source.as_str()),
            Some(&self.input_confirmation),
            Some(&self.evidence_refs),
        )
        .map_err(|_| SleepEnergyValidationError("invalid input_confirmation or evidence_refs"))?;

        if self.input_source == SleepEnergyInputSource::Typed && !self.evidence_refs.is_empty() {
            return Err(SleepEnergyValidationError(
                "typed sleep check-ins must not include evidence_refs",
            ));
        }

        match (self.input_confirmation.method, self.corrects_record_id) {
            (InputConfirmationMethod::ExplicitAcceptance, None) => {}
            (InputConfirmationMethod::ExplicitCorrection, Some(_)) => {}
            (InputConfirmationMethod::ExplicitAcceptance, Some(_)) => {
                return Err(SleepEnergyValidationError(
                    "corrects_record_id requires explicit_correction",
                ));
            }
            (InputConfirmationMethod::ExplicitCorrection, None) => {
                return Err(SleepEnergyValidationError(
                    "explicit_correction requires corrects_record_id",
                ));
            }
            _ => {
                return Err(SleepEnergyValidationError("invalid input_confirmation"));
            }
        }

        Ok(ConfirmedSleepEnergyCheckIn {
            local_date,
            sleep_duration_minutes: self.sleep_duration_minutes,
            sleep_start_local_time,
            sleep_end_local_time,
            daytime_energy: self.daytime_energy,
            caffeine_within_six_hours_of_sleep: self.caffeine_within_six_hours_of_sleep,
            screen_minutes_in_final_hour: self.screen_minutes_in_final_hour,
            input_source: self.input_source,
            input_confirmation: self.input_confirmation,
            corrects_record_id: self.corrects_record_id,
        })
    }
}

impl ConfirmedSleepEnergyCheckIn {
    pub fn canonical_text(&self) -> String {
        let mut lines = vec![
            format!(
                "Confirmed sleep and energy check-in for {}",
                self.local_date
            ),
            format!("Sleep duration: {} minutes", self.sleep_duration_minutes),
            format!("Daytime energy: {}/5", self.daytime_energy),
        ];
        if let (Some(start), Some(end)) = (self.sleep_start_local_time, self.sleep_end_local_time) {
            lines.push(format!(
                "Sleep timing: {}–{}",
                start.format("%H:%M"),
                end.format("%H:%M")
            ));
        }
        if let Some(caffeine) = self.caffeine_within_six_hours_of_sleep {
            lines.push(format!(
                "Caffeine within six hours: {}",
                if caffeine { "yes" } else { "no" }
            ));
        }
        if let Some(minutes) = self.screen_minutes_in_final_hour {
            lines.push(format!("Screen time in final hour: {minutes} minutes"));
        }
        lines.join("\n")
    }

    pub fn persistence_metadata(&self) -> Value {
        let mut record = serde_json::Map::from_iter([
            ("record_type".to_string(), json!("sleep_energy_check_in")),
            ("local_date".to_string(), json!(self.local_date.to_string())),
            (
                "sleep_duration_minutes".to_string(),
                json!(self.sleep_duration_minutes),
            ),
            ("daytime_energy".to_string(), json!(self.daytime_energy)),
            (
                "input_source".to_string(),
                json!(self.input_source.as_str()),
            ),
            (
                "input_confirmation".to_string(),
                json!(self.input_confirmation),
            ),
        ]);
        if let Some(start) = self.sleep_start_local_time {
            record.insert(
                "sleep_start_local_time".to_string(),
                json!(start.format("%H:%M").to_string()),
            );
        }
        if let Some(end) = self.sleep_end_local_time {
            record.insert(
                "sleep_end_local_time".to_string(),
                json!(end.format("%H:%M").to_string()),
            );
        }
        if let Some(caffeine) = self.caffeine_within_six_hours_of_sleep {
            record.insert(
                "caffeine_within_six_hours_of_sleep".to_string(),
                json!(caffeine),
            );
        }
        if let Some(minutes) = self.screen_minutes_in_final_hour {
            record.insert("screen_minutes_in_final_hour".to_string(), json!(minutes));
        }
        if let Some(corrects_record_id) = self.corrects_record_id {
            record.insert("corrects_record_id".to_string(), json!(corrects_record_id));
        }
        Value::Object(record)
    }

    pub fn trace_metadata(&self) -> Value {
        let mut metadata = serde_json::Map::from_iter([
            ("record_type".to_string(), json!("sleep_energy_check_in")),
            ("local_date".to_string(), json!(self.local_date.to_string())),
            (
                "input_source".to_string(),
                json!(self.input_source.as_str()),
            ),
            (
                "input_confirmation".to_string(),
                json!(self.input_confirmation),
            ),
            ("sleep_duration_present".to_string(), json!(true)),
            ("daytime_energy_present".to_string(), json!(true)),
            (
                "sleep_timing_present".to_string(),
                json!(self.sleep_start_local_time.is_some() && self.sleep_end_local_time.is_some()),
            ),
            (
                "caffeine_present".to_string(),
                json!(self.caffeine_within_six_hours_of_sleep.is_some()),
            ),
            (
                "screen_minutes_present".to_string(),
                json!(self.screen_minutes_in_final_hour.is_some()),
            ),
        ]);
        if let Some(corrects_record_id) = self.corrects_record_id {
            metadata.insert("corrects_record_id".to_string(), json!(corrects_record_id));
        }
        Value::Object(metadata)
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, SleepEnergyValidationError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| SleepEnergyValidationError("local_date must use ISO YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(SleepEnergyValidationError(
            "local_date must use ISO YYYY-MM-DD",
        ));
    }
    Ok(date)
}

fn parse_time(value: &str) -> Result<NaiveTime, SleepEnergyValidationError> {
    let time = NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| SleepEnergyValidationError("sleep times must use HH:MM"))?;
    if time.format("%H:%M").to_string() != value {
        return Err(SleepEnergyValidationError("sleep times must use HH:MM"));
    }
    Ok(time)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Value {
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 450,
            "sleep_start_local_time": "22:30",
            "sleep_end_local_time": "06:00",
            "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_acceptance"
            }
        })
    }

    #[test]
    fn accepts_confirmed_typed_and_agent_ocr_records() {
        let typed: SleepEnergyCheckInInput = serde_json::from_value(base()).unwrap();
        assert_eq!(
            typed.confirm().unwrap().input_source,
            SleepEnergyInputSource::Typed
        );

        let mut ocr = base();
        ocr["input_source"] = json!("agent_ocr");
        ocr["input_confirmation"]["method"] = json!("explicit_correction");
        ocr["corrects_record_id"] = json!(Uuid::new_v4());
        ocr["evidence_refs"] = json!([{
            "provider": "agent_ocr",
            "locator": "s3://private/check-in.png",
            "media_type": "image/png",
            "metadata": {"page": 1}
        }]);
        let ocr: SleepEnergyCheckInInput = serde_json::from_value(ocr).unwrap();
        assert_eq!(
            ocr.confirm().unwrap().input_source,
            SleepEnergyInputSource::AgentOcr
        );
    }

    #[test]
    fn rejects_unknown_confirmation_and_range_violations() {
        for (field, value) in [
            ("diagnosis", json!("anything")),
            ("metadata", json!({"anything": true})),
            ("raw_screenshot", json!("base64-not-allowed")),
            ("local_date", json!("13-07-2026")),
            ("sleep_duration_minutes", json!(59)),
            ("daytime_energy", json!(6)),
            ("screen_minutes_in_final_hour", json!(61)),
        ] {
            let mut value_to_parse = base();
            value_to_parse[field] = value;
            let parsed = serde_json::from_value::<SleepEnergyCheckInInput>(value_to_parse);
            assert!(
                parsed.is_err() || parsed.unwrap().confirm().is_err(),
                "{field} must be rejected"
            );
        }

        let mut missing_confirmation = base();
        missing_confirmation
            .as_object_mut()
            .unwrap()
            .remove("input_confirmation");
        assert!(serde_json::from_value::<SleepEnergyCheckInInput>(missing_confirmation).is_err());

        let mut typed_with_evidence = base();
        typed_with_evidence["evidence_refs"] = json!([{
            "provider": "agent_ocr",
            "locator": "s3://private/check-in.png",
            "media_type": "image/png",
            "metadata": {}
        }]);
        let typed_with_evidence: SleepEnergyCheckInInput =
            serde_json::from_value(typed_with_evidence).unwrap();
        assert!(typed_with_evidence.confirm().is_err());
    }

    #[test]
    fn correction_requires_explicit_correction_and_link() {
        let mut correction = base();
        correction["corrects_record_id"] = json!(Uuid::new_v4());
        let correction: SleepEnergyCheckInInput = serde_json::from_value(correction).unwrap();
        assert!(correction.confirm().is_err());
    }
}
