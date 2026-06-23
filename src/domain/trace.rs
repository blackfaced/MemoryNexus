use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceSourceType {
    Http,
    Cli,
    Mcp,
    Ui,
    Background,
    TestFixture,
}

impl TraceSourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Cli => "cli",
            Self::Mcp => "mcp",
            Self::Ui => "ui",
            Self::Background => "background",
            Self::TestFixture => "test_fixture",
        }
    }
}

impl std::fmt::Display for TraceSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceTaskType {
    Chat,
    Search,
    LensRun,
    Review,
    Practice,
    Feedback,
    Planning,
    Observation,
    Install,
    Profile,
    Routing,
    Consolidation,
    Dreaming,
}

impl TraceTaskType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Search => "search",
            Self::LensRun => "lens_run",
            Self::Review => "review",
            Self::Practice => "practice",
            Self::Feedback => "feedback",
            Self::Planning => "planning",
            Self::Observation => "observation",
            Self::Install => "install",
            Self::Profile => "profile",
            Self::Routing => "routing",
            Self::Consolidation => "consolidation",
            Self::Dreaming => "dreaming",
        }
    }
}

impl std::fmt::Display for TraceTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    Fast,
    Focused,
    Deep,
    None,
}

impl TraceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Focused => "focused",
            Self::Deep => "deep",
            Self::None => "none",
        }
    }
}

impl std::fmt::Display for TraceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceRuntime {
    Local,
    Cloud,
    Hybrid,
    Deterministic,
    Unknown,
}

impl TraceRuntime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
            Self::Hybrid => "hybrid",
            Self::Deterministic => "deterministic",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for TraceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Started,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

impl TraceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

impl std::fmt::Display for TraceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceGeneratedObjectLinks {
    #[serde(default)]
    pub memory_ids: Vec<Uuid>,
    #[serde(default)]
    pub lens_run_ids: Vec<Uuid>,
    #[serde(default)]
    pub review_report_ids: Vec<Uuid>,
    #[serde(default)]
    pub feedback_loop_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub source_type: TraceSourceType,
    pub task_type: TraceTaskType,
    pub mode: TraceMode,
    pub runtime: TraceRuntime,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<i64>,
    pub status: TraceStatus,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub token_usage: Option<Value>,
    pub estimated_cost_usd: Option<f64>,
    pub local_processing_ratio: Option<f64>,
    #[serde(default)]
    pub related_memory_ids: Vec<Uuid>,
    #[serde(default)]
    pub generated_memory_ids: Vec<Uuid>,
    #[serde(default)]
    pub generated_lens_run_ids: Vec<Uuid>,
    #[serde(default)]
    pub generated_review_report_ids: Vec<Uuid>,
    #[serde(default)]
    pub generated_feedback_loop_ids: Vec<Uuid>,
    pub user_feedback: Option<Value>,
    pub error: Option<Value>,
    #[serde(default)]
    pub metadata: Value,
}

impl Trace {
    pub fn completed(
        space_id: Uuid,
        namespace_id: Option<Uuid>,
        source_type: TraceSourceType,
        task_type: TraceTaskType,
        mode: TraceMode,
        runtime: TraceRuntime,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            space_id,
            namespace_id,
            source_type,
            task_type,
            mode,
            runtime,
            input_summary: None,
            output_summary: None,
            started_at: now,
            completed_at: Some(now),
            latency_ms: None,
            status: TraceStatus::Completed,
            model_provider: None,
            model_name: None,
            token_usage: None,
            estimated_cost_usd: None,
            local_processing_ratio: None,
            related_memory_ids: Vec::new(),
            generated_memory_ids: Vec::new(),
            generated_lens_run_ids: Vec::new(),
            generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: None,
            error: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_summaries(
        mut self,
        input_summary: Option<String>,
        output_summary: Option<String>,
    ) -> Self {
        self.input_summary = input_summary;
        self.output_summary = output_summary;
        self
    }

    pub fn with_latency_ms(mut self, latency_ms: i64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn with_generated_objects(mut self, links: TraceGeneratedObjectLinks) -> Self {
        self.generated_memory_ids = links.memory_ids;
        self.generated_lens_run_ids = links.lens_run_ids;
        self.generated_review_report_ids = links.review_report_ids;
        self.generated_feedback_loop_ids = links.feedback_loop_ids;
        self
    }
}
