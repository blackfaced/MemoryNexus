use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::ActorId;

pub type SurfaceTraceId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Surface {
    Capture,
    Performance,
    Reflection,
    Planning,
    Observation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceAction {
    CaptureObservation,
    SubmitAttempt,
    ReviewEvidence,
    GenerateNextTask,
    AdjustPlan,
    GetStateSummary,
    RequestConsolidation,
}

impl SurfaceAction {
    pub const fn surface(self) -> Surface {
        match self {
            Self::CaptureObservation => Surface::Capture,
            Self::SubmitAttempt => Surface::Performance,
            Self::ReviewEvidence => Surface::Reflection,
            Self::GenerateNextTask | Self::AdjustPlan => Surface::Planning,
            Self::GetStateSummary | Self::RequestConsolidation => Surface::Observation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceAdapter {
    Chat,
    Cli,
    Dashboard,
    Mcp,
    Mobile,
    Voice,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePreference {
    Auto,
    Cloud,
    Deterministic,
    Hybrid,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceContext {
    pub mode: Option<String>,
    pub locale: Option<String>,
    pub device: Option<String>,
    pub runtime_preference: Option<RuntimePreference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceRequest {
    pub namespace: String,
    pub surface: Surface,
    pub action: SurfaceAction,
    pub actor: ActorId,
    pub adapter: SurfaceAdapter,
    pub payload: Value,
    pub context: SurfaceContext,
}

impl SurfaceRequest {
    pub fn validate(&self) -> Result<(), SurfaceValidationError> {
        if self.action.surface() == self.surface {
            Ok(())
        } else {
            Err(SurfaceValidationError {
                surface: self.surface,
                action: self.action,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceValidationError {
    pub surface: Surface,
    pub action: SurfaceAction,
}

impl fmt::Display for SurfaceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "surface action {:?} is not valid for surface {:?}",
            self.action, self.surface
        )
    }
}

impl std::error::Error for SurfaceValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceVisibility {
    Adapter,
    Coach,
    Debug,
    Internal,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceResponse {
    pub surface: Surface,
    pub action: SurfaceAction,
    pub result: Value,
    pub generated_trace_id: SurfaceTraceId,
    pub follow_up_suggestions: Vec<String>,
    pub visibility: SurfaceVisibility,
}

impl SurfaceResponse {
    pub fn new(
        surface: Surface,
        action: SurfaceAction,
        result: Value,
        generated_trace_id: SurfaceTraceId,
        follow_up_suggestions: Vec<String>,
        visibility: SurfaceVisibility,
    ) -> Self {
        Self {
            surface,
            action,
            result,
            generated_trace_id,
            follow_up_suggestions,
            visibility,
        }
    }
}
