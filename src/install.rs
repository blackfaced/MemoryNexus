//! Profile-aware install planning for CLI and MCP self-install helpers.

use serde_json::{json, Value};
use std::fmt;

const DEFAULT_RELEASE_TAG: &str = "<release-tag>";
const DEFAULT_BIN_DIR: &str = "~/.local/bin";
const DEFAULT_API_URL: &str = "http://localhost:8080";
const RELEASE_BASE_URL: &str = "https://github.com/blackfaced/MemoryNexus/releases/download";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallProfile {
    Trial,
    LocalOneClick,
    Production,
    Developer,
}

impl InstallProfile {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "trial" => Ok(Self::Trial),
            "local-one-click" | "local" => Ok(Self::LocalOneClick),
            "production" => Ok(Self::Production),
            "developer" | "dev" => Ok(Self::Developer),
            _ => {
                Err("profile must be trial, local-one-click, production, or developer".to_string())
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trial => "trial",
            Self::LocalOneClick => "local-one-click",
            Self::Production => "production",
            Self::Developer => "developer",
        }
    }

    pub fn all() -> [Self; 4] {
        [
            Self::Trial,
            Self::LocalOneClick,
            Self::Production,
            Self::Developer,
        ]
    }
}

impl fmt::Display for InstallProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTarget {
    pub os: String,
    pub arch: String,
    pub target: Option<String>,
}

impl ReleaseTarget {
    pub fn detect() -> Self {
        release_target_for(std::env::consts::OS, std::env::consts::ARCH)
    }

    pub fn supported_for_test(os: &str, arch: &str, target: &str) -> Self {
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
            target: Some(target.to_string()),
        }
    }

    pub fn unsupported_for_test(os: &str, arch: &str) -> Self {
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
            target: None,
        }
    }

    pub fn supported(&self) -> bool {
        self.target.is_some()
    }

    pub fn target_or_placeholder(&self) -> String {
        self.target
            .clone()
            .unwrap_or_else(|| "<unsupported-target>".to_string())
    }
}

fn release_target_for(os: &str, arch: &str) -> ReleaseTarget {
    let target = match (os, arch) {
        ("macos", "aarch64" | "arm64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        _ => None,
    };

    ReleaseTarget {
        os: os.to_string(),
        arch: arch.to_string(),
        target: target.map(str::to_string),
    }
}

#[derive(Debug, Clone)]
pub struct InstallPlanOptions {
    pub api_url: String,
    pub release_tag: String,
    pub bin_dir: String,
    pub target: ReleaseTarget,
    pub binary_path: Option<String>,
    pub pull: bool,
    pub rebuild_mcp: bool,
    pub run_tests: bool,
    pub rebuild_api: bool,
}

impl InstallPlanOptions {
    pub fn new(
        api_url: impl Into<String>,
        release_tag: Option<String>,
        bin_dir: Option<String>,
        target: ReleaseTarget,
    ) -> Self {
        Self {
            api_url: api_url.into(),
            release_tag: release_tag.unwrap_or_else(|| DEFAULT_RELEASE_TAG.to_string()),
            bin_dir: bin_dir.unwrap_or_else(|| DEFAULT_BIN_DIR.to_string()),
            target,
            binary_path: None,
            pull: false,
            rebuild_mcp: false,
            run_tests: true,
            rebuild_api: false,
        }
    }

    pub fn for_test(release_tag: &str, target: &str) -> Self {
        Self::new(
            DEFAULT_API_URL,
            Some(release_tag.to_string()),
            Some("/tmp/memorynexus/bin".to_string()),
            ReleaseTarget::supported_for_test("test", "test", target),
        )
        .with_source_flags(false, true, true, false)
    }

    pub fn with_source_flags(
        mut self,
        pull: bool,
        rebuild_mcp: bool,
        run_tests: bool,
        rebuild_api: bool,
    ) -> Self {
        self.pull = pull;
        self.rebuild_mcp = rebuild_mcp;
        self.run_tests = run_tests;
        self.rebuild_api = rebuild_api;
        self
    }
}

#[derive(Debug, Clone)]
pub struct InstallStatusInput {
    pub selected_profile: Option<InstallProfile>,
    pub api_url: String,
    pub api_health: Value,
    pub local: Value,
    pub checkout: Option<Value>,
    pub release_tag: Option<String>,
    pub bin_dir: Option<String>,
    pub binary_path: Option<String>,
    pub target: ReleaseTarget,
}

pub fn install_status_value(input: InstallStatusInput) -> Value {
    let selected_profile = input.selected_profile.unwrap_or(InstallProfile::Trial);
    let target_report = input.target.clone();
    let release_tag = input
        .release_tag
        .clone()
        .unwrap_or_else(|| DEFAULT_RELEASE_TAG.to_string());
    let bin_dir = input
        .bin_dir
        .clone()
        .unwrap_or_else(|| DEFAULT_BIN_DIR.to_string());
    let target = target_report.target_or_placeholder();
    let archive_name = archive_name(&release_tag, &target);
    let binary_path = input.binary_path.clone().unwrap_or_else(|| {
        format!(
            "{}/{}",
            bin_dir.trim_end_matches('/'),
            match selected_profile {
                InstallProfile::Trial => "memorynexus-mcp",
                InstallProfile::LocalOneClick
                | InstallProfile::Production
                | InstallProfile::Developer => "memorynexus-mcp",
            }
        )
    });
    let fallback_reason = if target_report.supported() {
        None
    } else {
        Some(format!(
            "unsupported OS/arch {} {}; use Developer Profile source build or wait for a compatible release binary",
            target_report.os, target_report.arch
        ))
    };

    json!({
        "selected_profile": selected_profile.as_str(),
        "recommended_profile": selected_profile.as_str(),
        "profiles": profiles_summary(),
        "detected": {
            "os": target_report.os,
            "arch": target_report.arch,
        },
        "release": {
            "tag": release_tag.clone(),
            "target": target.clone(),
            "archive": archive_name.clone(),
            "archive_url": archive_url(&release_tag, &target),
            "checksum_url": checksum_url(&release_tag, &target),
        },
        "binary": {
            "path": binary_path.clone(),
            "available": target_report.supported(),
            "version": input.local.get("version").cloned().unwrap_or(Value::Null),
            "required_binaries": required_binaries(selected_profile),
        },
        "api": {
            "url": input.api_url.clone(),
            "health": input.api_health,
        },
        "mcp_smoke": {
            "initialize": mcp_smoke_command(&binary_path, true),
            "tools_list": mcp_smoke_command(&binary_path, false),
        },
        "checkout": input.checkout,
        "fallback": {
            "source_build_required": fallback_reason.is_some(),
            "reason": fallback_reason,
        },
        "plan": install_plan_value(
            selected_profile,
            InstallPlanOptions::new(
                input.api_url,
                Some(release_tag),
                Some(bin_dir),
                target_report,
            ),
        ),
    })
}

pub fn install_plan_value(profile: InstallProfile, options: InstallPlanOptions) -> Value {
    match profile {
        InstallProfile::Trial => trial_plan(options),
        InstallProfile::LocalOneClick => local_one_click_plan(options),
        InstallProfile::Production => production_plan(options),
        InstallProfile::Developer => developer_plan(options),
    }
}

pub fn profile_enum_json() -> Value {
    json!(InstallProfile::all()
        .iter()
        .map(|profile| profile.as_str())
        .collect::<Vec<_>>())
}

fn profiles_summary() -> Value {
    json!({
        "trial": {
            "summary": "Use prebuilt memorynexus-mcp with an existing hosted/demo API.",
            "requires": ["MEMORYNEXUS_API_URL", "MEMORYNEXUS_TOKEN", "compatible memorynexus-mcp binary"],
            "does_not_require": ["Rust", "cargo", "Docker", "PostgreSQL", "Qdrant"],
        },
        "local-one-click": {
            "summary": "Install release binaries and run local PostgreSQL/Qdrant through Docker.",
            "requires": ["release archive", "checksum", "Docker"],
            "does_not_require": ["Rust", "cargo", "rustup", "rustc"],
        },
        "production": {
            "summary": "Run release binaries against stable hosted or self-hosted services.",
            "requires": ["MemoryNexus API deployment", "PostgreSQL-compatible database", "Qdrant endpoint"],
            "does_not_require": ["Supabase specifically"],
        },
        "developer": {
            "summary": "Use the source checkout and cargo for contributors.",
            "requires": ["Rust", "cargo", "source checkout"],
        },
    })
}

fn required_binaries(profile: InstallProfile) -> Vec<&'static str> {
    match profile {
        InstallProfile::Trial => vec!["memorynexus-mcp"],
        InstallProfile::LocalOneClick | InstallProfile::Production => {
            vec!["memorynexus", "memorynexus-cli", "memorynexus-mcp"]
        }
        InstallProfile::Developer => vec!["target/debug/memorynexus-mcp"],
    }
}

fn trial_plan(options: InstallPlanOptions) -> Value {
    let target = options.target.target_or_placeholder();
    let archive = archive_name(&options.release_tag, &target);
    let mcp_binary = options
        .binary_path
        .unwrap_or_else(|| format!("{}/memorynexus-mcp", options.bin_dir.trim_end_matches('/')));

    json!({
        "profile": "trial",
        "mode": "binary-first-plan",
        "requires_source_build": !options.target.supported(),
        "fallback_reason": fallback_reason(&options.target),
        "steps": [
            {
                "command": "uname -s && uname -m",
                "reason": "detect OS and CPU architecture before selecting a release target",
            },
            {
                "command": format!("download or locate {archive} and extract memorynexus-mcp"),
                "reason": "Trial Profile only needs the MCP stdio binary because the API is hosted or managed separately",
            },
            {
                "command": format!("MEMORYNEXUS_API_URL={} MEMORYNEXUS_TOKEN=<token> {mcp_binary}", options.api_url),
                "reason": "configure the MCP server to call the hosted/demo Rust API",
            },
            {
                "command": mcp_smoke_command(&mcp_binary, true),
                "reason": "verify MCP initialize over stdio",
            },
            {
                "command": mcp_smoke_command(&mcp_binary, false),
                "reason": "verify MCP tools/list over stdio",
            },
        ],
        "notes": [
            "Trial Profile must not install a Rust toolchain, container runtime, local database, or local vector service.",
            "Use Developer Profile only when no compatible release binary exists or the user explicitly requests source build.",
        ],
    })
}

fn local_one_click_plan(options: InstallPlanOptions) -> Value {
    let target = options.target.target_or_placeholder();
    let archive = archive_name(&options.release_tag, &target);
    let archive_url = archive_url(&options.release_tag, &target);
    let checksum_url = checksum_url(&options.release_tag, &target);
    let bin_dir = options.bin_dir.trim_end_matches('/').to_string();
    let mcp_binary = format!("{bin_dir}/memorynexus-mcp");

    json!({
        "profile": "local-one-click",
        "mode": "binary-first-plan",
        "requires_source_build": !options.target.supported(),
        "fallback_reason": fallback_reason(&options.target),
        "release_archive": {
            "name": archive,
            "url": archive_url,
            "checksum_url": checksum_url,
        },
        "services": ["Docker services", "PostgreSQL", "Qdrant"],
        "steps": [
            {
                "command": "uname -s && uname -m",
                "reason": "detect OS and CPU architecture before selecting a release target",
            },
            {
                "command": format!("curl -fL -o {archive} {archive_url}"),
                "reason": "download the release archive containing memorynexus, memorynexus-cli, and memorynexus-mcp",
            },
            {
                "command": format!("curl -fL -o {archive}.sha256 {checksum_url} && sha256sum -c {archive}.sha256"),
                "reason": "verify release archive checksum before installing binaries",
            },
            {
                "command": format!("tar -xzf {archive} && install -m 0755 memorynexus-{}-{target}/memorynexus memorynexus-{}-{target}/memorynexus-cli memorynexus-{}-{target}/memorynexus-mcp {bin_dir}/", options.release_tag, options.release_tag, options.release_tag),
                "reason": "install the prebuilt binaries into the local bin directory",
            },
            {
                "command": "docker compose up -d postgres qdrant",
                "reason": "start or verify local PostgreSQL and Qdrant services",
            },
            {
                "command": format!("MEMORYNEXUS_API_URL={} memorynexus-cli health # checks /api/v1/health", options.api_url),
                "reason": "verify MemoryNexus API health",
            },
            {
                "command": mcp_config_command(&mcp_binary, &options.api_url),
                "reason": "write or output MCP config for the agent client",
            },
            {
                "command": mcp_smoke_command(&mcp_binary, false),
                "reason": "verify MCP tools/list over stdio",
            },
        ],
        "notes": [
            "Local One-click Profile must use release binaries instead of installing a Rust toolchain.",
            "Source build fallback is only for unsupported release targets or an explicit Developer Profile choice.",
        ],
    })
}

fn production_plan(options: InstallPlanOptions) -> Value {
    let target = options.target.target_or_placeholder();
    let archive = archive_name(&options.release_tag, &target);

    json!({
        "profile": "production",
        "mode": "binary-first-plan",
        "requires_source_build": !options.target.supported(),
        "fallback_reason": fallback_reason(&options.target),
        "steps": [
            {
                "command": format!("download and verify {archive} plus {archive}.sha256"),
                "reason": "use release binaries as service artifacts",
            },
            {
                "command": "configure DATABASE_URL, JWT_SECRET, QDRANT_URL, and optional provider keys",
                "reason": "Production Profile uses stable hosted or self-hosted services and is not Supabase-only",
            },
            {
                "command": "memorynexus",
                "reason": "run the Rust API binary as a service or container entrypoint",
            },
            {
                "command": format!("MEMORYNEXUS_API_URL={} memorynexus-cli health", options.api_url),
                "reason": "verify API health from the CLI",
            },
            {
                "command": mcp_smoke_command("memorynexus-mcp", false),
                "reason": "verify MCP tools/list through the production API URL",
            },
        ],
    })
}

fn developer_plan(options: InstallPlanOptions) -> Value {
    let mut steps = Vec::new();
    steps.push(json!({
        "command": "git status --short",
        "reason": "detect local edits before any source update",
    }));
    if options.pull {
        steps.push(json!({
            "command": "git pull",
            "reason": "update source from the configured remote; skipped when local edits are already the desired upgrade",
        }));
    }
    if options.run_tests {
        steps.push(json!({
            "command": "cargo test",
            "reason": "verify the updated checkout before reconnecting agents",
        }));
    }
    if options.rebuild_mcp {
        steps.push(json!({
            "command": "cargo build --bin memorynexus-mcp",
            "reason": "refresh built-binary MCP installs",
        }));
    }
    if options.rebuild_api {
        steps.push(json!({
            "command": "cargo build --bin memorynexus",
            "reason": "refresh built-binary API installs",
        }));
    }
    steps.push(json!({
        "command": "restart API and reload MCP client",
        "reason": "running processes keep old code until restarted",
    }));

    json!({
        "profile": "developer",
        "mode": "source-build-plan",
        "requires_source_build": true,
        "steps": steps,
        "notes": [
            "Developer Profile is the contributor path and intentionally keeps cargo build/test.",
            "Skip git pull when the checkout already contains the user's local edits.",
            "Restart the API after backend code or migrations change; migrations run on API startup."
        ],
    })
}

fn fallback_reason(target: &ReleaseTarget) -> Value {
    if target.supported() {
        Value::Null
    } else {
        json!(format!(
            "unsupported OS/arch {} {}; fall back to Developer Profile source build only when the user accepts that path",
            target.os, target.arch
        ))
    }
}

fn archive_name(release_tag: &str, target: &str) -> String {
    format!("memorynexus-{release_tag}-{target}.tar.gz")
}

fn archive_url(release_tag: &str, target: &str) -> String {
    format!(
        "{RELEASE_BASE_URL}/{release_tag}/{}",
        archive_name(release_tag, target)
    )
}

fn checksum_url(release_tag: &str, target: &str) -> String {
    format!("{}.sha256", archive_url(release_tag, target))
}

fn mcp_smoke_command(binary: &str, initialize: bool) -> String {
    let payload = if initialize {
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#
    } else {
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#
    };
    format!(
        "printf '%s\\n' '{}' | MEMORYNEXUS_API_URL=$MEMORYNEXUS_API_URL MEMORYNEXUS_TOKEN=$MEMORYNEXUS_TOKEN {}",
        payload, binary
    )
}

fn mcp_config_command(binary: &str, api_url: &str) -> String {
    format!(
        "write MCP config with command={binary} and env MEMORYNEXUS_API_URL={api_url}, MEMORYNEXUS_TOKEN=<token>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_release_targets() {
        assert_eq!(
            release_target_for("macos", "aarch64").target.as_deref(),
            Some("aarch64-apple-darwin")
        );
        assert_eq!(
            release_target_for("macos", "x86_64").target.as_deref(),
            Some("x86_64-apple-darwin")
        );
        assert_eq!(
            release_target_for("linux", "x86_64").target.as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert!(release_target_for("linux", "aarch64").target.is_none());
    }
}
