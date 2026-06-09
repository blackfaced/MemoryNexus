use serde_json::{json, Value};

const DEFAULT_API_URL: &str = "http://localhost:8080";

#[derive(Debug, Clone, Copy)]
struct LensTemplate {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    strategy: &'static str,
    output_format: &'static str,
    retrieval_mode: &'static str,
}

const LENS_TEMPLATES: &[LensTemplate] = &[
    LensTemplate {
        id: "project_context",
        name: "Project Context",
        description: "Interpret project memories for planning and direction.",
        strategy: "project_context",
        output_format: "brief",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "learning_review",
        name: "Learning Review",
        description: "Review learning memories and extract progress, gaps, and next steps.",
        strategy: "learning_review",
        output_format: "bullets",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "family_growth",
        name: "Family Growth",
        description: "Interpret family memories as growth moments and continuity signals.",
        strategy: "family_growth",
        output_format: "brief",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "risk_review",
        name: "Risk Review",
        description: "Read memories through risks, contradictions, and unresolved concerns.",
        strategy: "risk_review",
        output_format: "bullets",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "personal_context",
        name: "Personal Context",
        description: "Interpret personal memories for an agent's working context.",
        strategy: "personal_context",
        output_format: "brief",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "preference_review",
        name: "Preference Review",
        description: "Extract stable user preferences, dislikes, and working style signals.",
        strategy: "preference_review",
        output_format: "bullets",
        retrieval_mode: "semantic",
    },
    LensTemplate {
        id: "decision_history",
        name: "Decision History",
        description: "Review past decisions, rationale, reversals, and open tradeoffs.",
        strategy: "decision_history",
        output_format: "bullets",
        retrieval_mode: "semantic",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Health,
    Config,
    Version,
    Completion {
        shell: Shell,
    },
    InstallStatus {
        checkout_dir: Option<String>,
        profile: Option<memorynexus::install::InstallProfile>,
    },
    Upgrade {
        checkout_dir: Option<String>,
        profile: memorynexus::install::InstallProfile,
        apply: bool,
        pull: bool,
        rebuild_mcp: bool,
        rebuild_api: bool,
        skip_tests: bool,
    },
    AuthRegister {
        email: String,
        username: String,
        password: String,
    },
    AuthLogin {
        email: String,
        password: String,
    },
    SpaceCreate {
        name: String,
        description: Option<String>,
    },
    SpaceList,
    FamilyCreate {
        name: String,
        description: Option<String>,
    },
    FamilyList,
    FamilyMembers {
        space_id: String,
    },
    FamilyInvite {
        space_id: String,
        role: String,
        expires_in_days: Option<usize>,
    },
    FamilyAccept {
        code: String,
    },
    FamilyRole {
        space_id: String,
        user_id: String,
        role: String,
    },
    LensCreate {
        space_id: String,
        name: String,
        description: Option<String>,
        strategy: String,
        output_format: String,
        retrieval_mode: String,
    },
    LensTemplates,
    LensList {
        space_id: String,
    },
    LensGet {
        id: String,
    },
    LensRun {
        lens_id: String,
        query: String,
        limit: usize,
    },
    LensRunGet {
        id: String,
    },
    LensRunList {
        lens_id: Option<String>,
        space_id: Option<String>,
        limit: usize,
    },
    MemoryAdd {
        space_id: Option<String>,
        content: String,
        title: Option<String>,
        tags: Vec<String>,
        memory_type: String,
        is_shared: bool,
    },
    MemoryList {
        space_id: Option<String>,
        limit: usize,
        offset: usize,
    },
    MemoryGet {
        id: String,
    },
    MemoryDelete {
        id: String,
    },
    ReminderAdd {
        space_id: String,
        content: String,
        remind_at: String,
        title: Option<String>,
        memory_id: Option<String>,
        repeat_rule: Option<String>,
        delivery_channel: Option<String>,
    },
    ReminderList {
        space_id: String,
        due_only: bool,
        include_completed: bool,
        limit: usize,
    },
    ReminderComplete {
        id: String,
    },
    ReminderDelivery {
        id: String,
        status: String,
        error: Option<String>,
    },
    ReviewCreate {
        space_id: String,
        lens_id: String,
        window_start: String,
        window_end: String,
        report_type: String,
        limit: usize,
    },
    ReviewGet {
        id: String,
    },
    ReviewList {
        space_id: String,
        lens_id: Option<String>,
        limit: usize,
    },
    Search {
        space_id: Option<String>,
        lens_id: Option<String>,
        query: String,
        semantic: bool,
        limit: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Human,
}

impl OutputFormat {
    fn parse(value: &str, flag: &str) -> Result<Self, CliError> {
        match value {
            "json" => Ok(Self::Json),
            "human" => Ok(Self::Human),
            _ => Err(CliError::new(format!("{flag} must be json or human"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            _ => Err(CliError::new("shell must be bash, zsh, or fish")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliInvocation {
    command: Command,
    output_format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    api_url: String,
    token: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            api_url: std::env::var("MEMORYNEXUS_API_URL")
                .unwrap_or_else(|_| DEFAULT_API_URL.to_string()),
            token: std::env::var("MEMORYNEXUS_TOKEN").ok(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequestSpec {
    method: HttpMethod,
    url: String,
    body: Option<Value>,
    token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "ok": false,
            "error": {
                "message": self.message,
            }
        })
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

#[tokio::main]
async fn main() {
    let invocation = parse_cli(std::env::args());
    let result = match invocation {
        Ok(invocation) => execute(&Config::from_env(), &invocation.command)
            .await
            .map(|value| render_output(&value, invocation.output_format)),
        Err(error) => Err(error),
    };

    match result {
        Ok(output) => {
            println!("{}", output);
        }
        Err(error) => {
            eprintln!("{}", error.to_json());
            std::process::exit(1);
        }
    }
}

fn parse_cli<I, S>(args: I) -> Result<CliInvocation, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string())
        .collect();
    let mut output_format = OutputFormat::Json;
    let mut output_format_was_set = false;

    if matches!(
        args.get(1).map(String::as_str),
        Some("--output" | "--format")
    ) {
        let flag = args[1].clone();
        let value = args
            .get(2)
            .ok_or_else(|| CliError::new(format!("{flag} requires json or human")))?;
        output_format = OutputFormat::parse(value, &flag)?;
        output_format_was_set = true;
        args.drain(1..=2);
    }

    if args.iter().skip(2).any(|arg| arg == "--format") {
        return Err(CliError::new("--format must be a leading global flag"));
    }

    let command = parse_command(args)?;
    if matches!(command, Command::Completion { .. }) && !output_format_was_set {
        output_format = OutputFormat::Human;
    }

    Ok(CliInvocation {
        command,
        output_format,
    })
}

fn parse_command<I, S>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string())
        .collect();
    let Some(command) = args.get(1).map(String::as_str) else {
        return Err(CliError::new(usage()));
    };

    match command {
        "health" => Ok(Command::Health),
        "config" => Ok(Command::Config),
        "version" => Ok(Command::Version),
        "completion" => parse_completion_command(&args[2..]),
        "install" => parse_install_command(&args[2..]),
        "upgrade" => parse_upgrade_command(&args[2..]),
        "auth" => parse_auth_command(&args[2..]),
        "space" => parse_space_command(&args[2..]),
        "family" => parse_family_command(&args[2..]),
        "lens" => parse_lens_command(&args[2..]),
        "memory" => parse_memory_command(&args[2..]),
        "reminder" | "remind" => parse_reminder_command(&args[2..]),
        "review" => parse_review_command(&args[2..]),
        "search" => parse_search_command(&args[2..]),
        _ => Err(CliError::new(usage())),
    }
}

fn parse_completion_command(args: &[String]) -> Result<Command, CliError> {
    let shell = args
        .first()
        .filter(|shell| !shell.starts_with("--"))
        .ok_or_else(|| CliError::new("completion shell is required"))?;
    Ok(Command::Completion {
        shell: Shell::parse(shell)?,
    })
}

fn parse_install_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("install subcommand is required"));
    };

    match subcommand {
        "status" => Ok(Command::InstallStatus {
            checkout_dir: optional_flag(args, "--checkout"),
            profile: optional_flag(args, "--profile")
                .map(|value| memorynexus::install::InstallProfile::parse(&value))
                .transpose()
                .map_err(CliError::new)?,
        }),
        _ => Err(CliError::new("unknown install subcommand")),
    }
}

fn parse_upgrade_command(args: &[String]) -> Result<Command, CliError> {
    Ok(Command::Upgrade {
        checkout_dir: optional_flag(args, "--checkout"),
        profile: optional_flag(args, "--profile")
            .map(|value| memorynexus::install::InstallProfile::parse(&value))
            .transpose()
            .map_err(CliError::new)?
            .unwrap_or(memorynexus::install::InstallProfile::Developer),
        apply: has_flag(args, "--apply"),
        pull: has_flag(args, "--pull"),
        rebuild_mcp: has_flag(args, "--rebuild-mcp"),
        rebuild_api: has_flag(args, "--rebuild-api"),
        skip_tests: has_flag(args, "--skip-tests"),
    })
}

fn parse_auth_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("auth subcommand is required"));
    };

    match subcommand {
        "register" => {
            let email = required_flag(args, "--email")?;
            let username = optional_flag(args, "--username")
                .or_else(|| optional_flag(args, "--name"))
                .ok_or_else(|| CliError::new("--username is required"))?;
            let password = required_flag(args, "--password")?;
            Ok(Command::AuthRegister {
                email,
                username,
                password,
            })
        }
        "login" => Ok(Command::AuthLogin {
            email: required_flag(args, "--email")?,
            password: required_flag(args, "--password")?,
        }),
        _ => Err(CliError::new("unknown auth subcommand")),
    }
}

fn parse_space_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("space subcommand is required"));
    };

    match subcommand {
        "create" => Ok(Command::SpaceCreate {
            name: required_flag(args, "--name")?,
            description: optional_flag(args, "--description"),
        }),
        "list" => Ok(Command::SpaceList),
        _ => Err(CliError::new("unknown space subcommand")),
    }
}

fn parse_family_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("family subcommand is required"));
    };

    match subcommand {
        "create" => Ok(Command::FamilyCreate {
            name: required_flag(args, "--name")?,
            description: optional_flag(args, "--description"),
        }),
        "list" => Ok(Command::FamilyList),
        "members" => Ok(Command::FamilyMembers {
            space_id: required_flag(args, "--space")?,
        }),
        "invite" => Ok(Command::FamilyInvite {
            space_id: required_flag(args, "--space")?,
            role: optional_flag(args, "--role").unwrap_or_else(|| "viewer".to_string()),
            expires_in_days: optional_flag(args, "--expires-in-days")
                .map(|value| {
                    value
                        .parse::<usize>()
                        .map_err(|_| CliError::new("--expires-in-days must be a positive integer"))
                })
                .transpose()?,
        }),
        "accept" => Ok(Command::FamilyAccept {
            code: required_flag(args, "--code")?,
        }),
        "role" => Ok(Command::FamilyRole {
            space_id: required_flag(args, "--space")?,
            user_id: required_flag(args, "--user")?,
            role: required_flag(args, "--role")?,
        }),
        _ => Err(CliError::new("unknown family subcommand")),
    }
}

fn parse_lens_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("lens subcommand is required"));
    };

    match subcommand {
        "templates" => Ok(Command::LensTemplates),
        "create" => parse_lens_create_command(args),
        "list" => Ok(Command::LensList {
            space_id: required_flag(args, "--space")?,
        }),
        "get" => Ok(Command::LensGet {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("lens id is required"))?,
        }),
        "run" => parse_lens_run_command(&args[1..]),
        _ => Err(CliError::new("unknown lens subcommand")),
    }
}

fn parse_lens_create_command(args: &[String]) -> Result<Command, CliError> {
    let template = optional_flag(args, "--template")
        .map(|id| {
            lens_template(&id)
                .copied()
                .ok_or_else(|| CliError::new(format!("unknown lens template: {id}")))
        })
        .transpose()?;

    Ok(Command::LensCreate {
        space_id: required_flag(args, "--space")?,
        name: optional_flag(args, "--name")
            .or_else(|| template.map(|template| template.name.to_string()))
            .ok_or_else(|| CliError::new("--name is required"))?,
        description: optional_flag(args, "--description")
            .or_else(|| template.map(|template| template.description.to_string())),
        strategy: optional_flag(args, "--strategy")
            .or_else(|| template.map(|template| template.strategy.to_string()))
            .unwrap_or_else(|| "default".to_string()),
        output_format: optional_flag(args, "--output")
            .or_else(|| template.map(|template| template.output_format.to_string()))
            .unwrap_or_else(|| "summary".to_string()),
        retrieval_mode: optional_flag(args, "--retrieval")
            .or_else(|| template.map(|template| template.retrieval_mode.to_string()))
            .unwrap_or_else(|| "semantic".to_string()),
    })
}

fn lens_template(id: &str) -> Option<&'static LensTemplate> {
    LENS_TEMPLATES.iter().find(|template| template.id == id)
}

fn parse_lens_run_command(args: &[String]) -> Result<Command, CliError> {
    if args.first().map(String::as_str) == Some("list") {
        let lens_id = optional_flag(args, "--lens");
        let space_id = optional_flag(args, "--space");
        if lens_id.is_none() && space_id.is_none() {
            return Err(CliError::new("--lens or --space is required"));
        }
        return Ok(Command::LensRunList {
            lens_id,
            space_id,
            limit: parse_usize_flag(args, "--limit", 20)?,
        });
    }

    if args.first().map(String::as_str) == Some("get") {
        return Ok(Command::LensRunGet {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("lens run id is required"))?,
        });
    }

    Ok(Command::LensRun {
        lens_id: args
            .first()
            .filter(|id| !id.starts_with("--"))
            .cloned()
            .ok_or_else(|| CliError::new("lens id is required"))?,
        query: required_flag(args, "--query")?,
        limit: parse_usize_flag(args, "--limit", 5)?,
    })
}

fn parse_memory_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("memory subcommand is required"));
    };

    match subcommand {
        "add" => Ok(Command::MemoryAdd {
            space_id: optional_flag(args, "--space"),
            content: required_flag(args, "--content")?,
            title: optional_flag(args, "--title"),
            tags: optional_flag(args, "--tags")
                .map(|tags| parse_tags(&tags))
                .unwrap_or_default(),
            memory_type: optional_flag(args, "--type").unwrap_or_else(|| "text".to_string()),
            is_shared: has_flag(args, "--shared"),
        }),
        "list" => Ok(Command::MemoryList {
            space_id: optional_flag(args, "--space"),
            limit: parse_usize_flag(args, "--limit", 20)?,
            offset: parse_usize_flag(args, "--offset", 0)?,
        }),
        "get" => Ok(Command::MemoryGet {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("memory id is required"))?,
        }),
        "delete" => Ok(Command::MemoryDelete {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("memory id is required"))?,
        }),
        _ => Err(CliError::new("unknown memory subcommand")),
    }
}

fn parse_reminder_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("reminder subcommand is required"));
    };

    match subcommand {
        "add" => Ok(Command::ReminderAdd {
            space_id: required_flag(args, "--space")?,
            content: required_flag(args, "--content")?,
            remind_at: required_flag(args, "--at")?,
            title: optional_flag(args, "--title"),
            memory_id: optional_flag(args, "--memory"),
            repeat_rule: optional_flag(args, "--repeat"),
            delivery_channel: optional_flag(args, "--channel"),
        }),
        "list" => Ok(Command::ReminderList {
            space_id: required_flag(args, "--space")?,
            due_only: has_flag(args, "--due"),
            include_completed: has_flag(args, "--include-completed"),
            limit: parse_usize_flag(args, "--limit", 20)?,
        }),
        "complete" => Ok(Command::ReminderComplete {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("reminder id is required"))?,
        }),
        "delivery" => Ok(Command::ReminderDelivery {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("reminder id is required"))?,
            status: required_flag(args, "--status")?,
            error: optional_flag(args, "--error"),
        }),
        _ => Err(CliError::new("unknown reminder subcommand")),
    }
}

fn parse_review_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("review subcommand is required"));
    };

    match subcommand {
        "create" => Ok(Command::ReviewCreate {
            space_id: required_flag(args, "--space")?,
            lens_id: required_flag(args, "--lens")?,
            window_start: required_flag(args, "--from")?,
            window_end: required_flag(args, "--to")?,
            report_type: optional_flag(args, "--type")
                .unwrap_or_else(|| "periodic_review".to_string()),
            limit: parse_usize_flag(args, "--limit", 30)?,
        }),
        "get" => Ok(Command::ReviewGet {
            id: args
                .get(1)
                .filter(|id| !id.starts_with("--"))
                .cloned()
                .ok_or_else(|| CliError::new("review report id is required"))?,
        }),
        "list" => Ok(Command::ReviewList {
            space_id: required_flag(args, "--space")?,
            lens_id: optional_flag(args, "--lens"),
            limit: parse_usize_flag(args, "--limit", 20)?,
        }),
        _ => Err(CliError::new("unknown review subcommand")),
    }
}

fn parse_search_command(args: &[String]) -> Result<Command, CliError> {
    let query = args
        .first()
        .filter(|query| !query.starts_with("--"))
        .cloned()
        .ok_or_else(|| CliError::new("search query is required"))?;

    Ok(Command::Search {
        space_id: optional_flag(args, "--space"),
        lens_id: optional_flag(args, "--lens"),
        query,
        semantic: has_flag(args, "--semantic"),
        limit: parse_usize_flag(args, "--limit", 20)?,
    })
}

fn build_request(config: &Config, command: &Command) -> Result<RequestSpec, CliError> {
    let base_url = config.api_url.trim_end_matches('/');

    match command {
        Command::Health => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/health"),
            body: None,
            token: None,
        }),
        Command::Config => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/ai/config"),
            body: None,
            token: None,
        }),
        Command::Version
        | Command::Completion { .. }
        | Command::InstallStatus { .. }
        | Command::Upgrade { .. } => Err(CliError::new("local command has no HTTP request")),
        Command::AuthRegister {
            email,
            username,
            password,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/auth/register"),
            body: Some(json!({
                "email": email,
                "username": username,
                "password": password,
            })),
            token: None,
        }),
        Command::AuthLogin { email, password } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/auth/login"),
            body: Some(json!({
                "email": email,
                "password": password,
            })),
            token: None,
        }),
        Command::SpaceCreate { name, description } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/spaces"),
            body: Some(json!({
                "name": name,
                "description": description,
            })),
            token: Some(require_token(config)?),
        }),
        Command::SpaceList => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/spaces"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::FamilyCreate { name, description } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/spaces"),
            body: Some(json!({
                "name": name,
                "description": description,
                "space_type": "family",
            })),
            token: Some(require_token(config)?),
        }),
        Command::FamilyList => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: with_query(
                &format!("{base_url}/api/v1/spaces"),
                &[("space_type", "family")],
            )?,
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::FamilyMembers { space_id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/spaces/{space_id}/members"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::FamilyInvite {
            space_id,
            role,
            expires_in_days,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/spaces/{space_id}/invites"),
            body: Some(json!({
                "role": role,
                "expires_in_days": expires_in_days,
            })),
            token: Some(require_token(config)?),
        }),
        Command::FamilyAccept { code } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/spaces/invites/accept"),
            body: Some(json!({
                "code": code,
            })),
            token: Some(require_token(config)?),
        }),
        Command::FamilyRole {
            space_id,
            user_id,
            role,
        } => Ok(RequestSpec {
            method: HttpMethod::Patch,
            url: format!("{base_url}/api/v1/spaces/{space_id}/members/{user_id}"),
            body: Some(json!({
                "role": role,
            })),
            token: Some(require_token(config)?),
        }),
        Command::LensCreate {
            space_id,
            name,
            description,
            strategy,
            output_format,
            retrieval_mode,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/lenses"),
            body: Some(json!({
                "space_id": space_id,
                "name": name,
                "description": description,
                "strategy": strategy,
                "output_format": output_format,
                "retrieval_mode": retrieval_mode,
            })),
            token: Some(require_token(config)?),
        }),
        Command::LensList { space_id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: with_query(
                &format!("{base_url}/api/v1/lenses"),
                &[("space_id", space_id.as_str())],
            )?,
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::LensGet { id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/lenses/{id}"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::LensTemplates => Err(CliError::new("lens templates is a local command")),
        Command::LensRun {
            lens_id,
            query,
            limit,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/lens-runs"),
            body: Some(json!({
                "lens_id": lens_id,
                "query": query,
                "limit": limit,
            })),
            token: Some(require_token(config)?),
        }),
        Command::LensRunGet { id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/lens-runs/{id}"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::LensRunList {
            lens_id,
            space_id,
            limit,
        } => {
            let limit = limit.to_string();
            let mut pairs = vec![("limit", limit.as_str())];
            if let Some(lens_id) = lens_id {
                pairs.push(("lens_id", lens_id.as_str()));
            }
            if let Some(space_id) = space_id {
                pairs.push(("space_id", space_id.as_str()));
            }
            Ok(RequestSpec {
                method: HttpMethod::Get,
                url: with_query(&format!("{base_url}/api/v1/lens-runs"), &pairs)?,
                body: None,
                token: Some(require_token(config)?),
            })
        }
        Command::MemoryAdd {
            space_id,
            content,
            title,
            tags,
            memory_type,
            is_shared,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/memories"),
            body: Some(json!({
                "title": title,
                "content": content,
                "memory_type": memory_type,
                "tags": tags,
                "is_shared": is_shared,
                "space_id": space_id,
            })),
            token: Some(require_token(config)?),
        }),
        Command::MemoryList {
            space_id,
            limit,
            offset,
        } => {
            let limit = limit.to_string();
            let offset = offset.to_string();
            let mut pairs = vec![("limit", limit.as_str()), ("offset", offset.as_str())];
            if let Some(space_id) = space_id {
                pairs.push(("space_id", space_id.as_str()));
            }
            Ok(RequestSpec {
                method: HttpMethod::Get,
                url: with_query(&format!("{base_url}/api/v1/memories"), &pairs)?,
                body: None,
                token: Some(require_token(config)?),
            })
        }
        Command::MemoryGet { id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/memories/{id}"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::MemoryDelete { id } => Ok(RequestSpec {
            method: HttpMethod::Delete,
            url: format!("{base_url}/api/v1/memories/{id}"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::ReminderAdd {
            space_id,
            content,
            remind_at,
            title,
            memory_id,
            repeat_rule,
            delivery_channel,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/reminders"),
            body: Some(json!({
                "space_id": space_id,
                "memory_id": memory_id,
                "title": title,
                "content": content,
                "remind_at": remind_at,
                "repeat_rule": repeat_rule,
                "delivery_channel": delivery_channel,
            })),
            token: Some(require_token(config)?),
        }),
        Command::ReminderList {
            space_id,
            due_only,
            include_completed,
            limit,
        } => {
            let due_only = due_only.to_string();
            let include_completed = include_completed.to_string();
            let limit = limit.to_string();
            Ok(RequestSpec {
                method: HttpMethod::Get,
                url: with_query(
                    &format!("{base_url}/api/v1/reminders"),
                    &[
                        ("space_id", space_id.as_str()),
                        ("due_only", due_only.as_str()),
                        ("include_completed", include_completed.as_str()),
                        ("limit", limit.as_str()),
                    ],
                )?,
                body: None,
                token: Some(require_token(config)?),
            })
        }
        Command::ReminderComplete { id } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/reminders/{id}/complete"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::ReminderDelivery { id, status, error } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/reminders/{id}/delivery"),
            body: Some(json!({
                "status": status,
                "error": error,
            })),
            token: Some(require_token(config)?),
        }),
        Command::ReviewCreate {
            space_id,
            lens_id,
            window_start,
            window_end,
            report_type,
            limit,
        } => Ok(RequestSpec {
            method: HttpMethod::Post,
            url: format!("{base_url}/api/v1/review-reports"),
            body: Some(json!({
                "space_id": space_id,
                "lens_id": lens_id,
                "window_start": window_start,
                "window_end": window_end,
                "report_type": report_type,
                "limit": limit,
            })),
            token: Some(require_token(config)?),
        }),
        Command::ReviewGet { id } => Ok(RequestSpec {
            method: HttpMethod::Get,
            url: format!("{base_url}/api/v1/review-reports/{id}"),
            body: None,
            token: Some(require_token(config)?),
        }),
        Command::ReviewList {
            space_id,
            lens_id,
            limit,
        } => {
            let limit = limit.to_string();
            let mut pairs = vec![("space_id", space_id.as_str()), ("limit", limit.as_str())];
            if let Some(lens_id) = lens_id {
                pairs.push(("lens_id", lens_id.as_str()));
            }
            Ok(RequestSpec {
                method: HttpMethod::Get,
                url: with_query(&format!("{base_url}/api/v1/review-reports"), &pairs)?,
                body: None,
                token: Some(require_token(config)?),
            })
        }
        Command::Search {
            space_id,
            lens_id,
            query,
            semantic,
            limit,
        } => {
            let semantic = semantic.to_string();
            let limit = limit.to_string();
            let mut pairs = vec![
                ("q", query.as_str()),
                ("semantic", semantic.as_str()),
                ("limit", limit.as_str()),
            ];
            if let Some(space_id) = space_id {
                pairs.push(("space_id", space_id.as_str()));
            }
            if let Some(lens_id) = lens_id {
                pairs.push(("lens_id", lens_id.as_str()));
            }
            Ok(RequestSpec {
                method: HttpMethod::Get,
                url: with_query(&format!("{base_url}/api/v1/search"), &pairs)?,
                body: None,
                token: Some(require_token(config)?),
            })
        }
    }
}

async fn execute(config: &Config, command: &Command) -> Result<Value, CliError> {
    match command {
        Command::LensTemplates => return Ok(lens_templates_response()),
        Command::Version => return Ok(local_version_response()),
        Command::Completion { shell } => {
            return Ok(json!({
                "ok": true,
                "data": {
                    "script": completion_script(*shell),
                }
            }));
        }
        Command::InstallStatus {
            checkout_dir,
            profile,
        } => {
            return install_status_response(config, checkout_dir.as_deref(), *profile).await;
        }
        Command::Upgrade {
            checkout_dir,
            profile,
            apply,
            pull,
            rebuild_mcp,
            rebuild_api,
            skip_tests,
        } => {
            return upgrade_response(
                checkout_dir.as_deref(),
                *profile,
                *apply,
                *pull,
                *rebuild_mcp,
                *rebuild_api,
                *skip_tests,
            );
        }
        _ => {}
    }

    let request = build_request(config, command)?;
    let client = reqwest::Client::new();
    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
        HttpMethod::Patch => client.patch(&request.url),
        HttpMethod::Delete => client.delete(&request.url),
    };

    if let Some(token) = request.token {
        builder = builder.bearer_auth(token);
    }

    if let Some(body) = request.body {
        builder = builder.json(&body);
    }

    let response = builder
        .send()
        .await
        .map_err(|error| CliError::new(error.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| CliError::new(error.to_string()))?;
    let value = if text.trim().is_empty() {
        json!({ "ok": true })
    } else {
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }))
    };

    if !status.is_success() {
        return Err(CliError::new(format!(
            "HTTP {}: {}",
            status.as_u16(),
            value
        )));
    }

    Ok(value)
}

fn lens_templates_response() -> Value {
    json!({
        "ok": true,
        "data": {
            "items": LENS_TEMPLATES
                .iter()
                .map(|template| json!({
                    "id": template.id,
                    "name": template.name,
                    "description": template.description,
                    "strategy": template.strategy,
                    "output_format": template.output_format,
                    "retrieval_mode": template.retrieval_mode,
                }))
                .collect::<Vec<_>>()
        }
    })
}

fn local_version_response() -> Value {
    json!({
        "ok": true,
        "data": local_version_data(),
    })
}

fn local_version_data() -> Value {
    json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "binary": std::env::current_exe()
            .ok()
            .map(|path| path.display().to_string()),
    })
}

async fn install_status_response(
    config: &Config,
    checkout_dir: Option<&str>,
    profile: Option<memorynexus::install::InstallProfile>,
) -> Result<Value, CliError> {
    let checkout = checkout_dir
        .map(|path| resolve_checkout_dir(Some(path)))
        .transpose()?
        .map(|checkout| checkout_status(&checkout));
    let api_health = fetch_api_health(config).await.unwrap_or_else(|error| {
        json!({
            "reachable": false,
            "error": error.message,
        })
    });
    let target = memorynexus::install::ReleaseTarget::detect();
    let data =
        memorynexus::install::install_status_value(memorynexus::install::InstallStatusInput {
            selected_profile: profile,
            api_url: config.api_url.clone(),
            api_health,
            local: local_version_data(),
            checkout,
            release_tag: None,
            bin_dir: None,
            binary_path: std::env::current_exe()
                .ok()
                .map(|path| path.display().to_string()),
            target,
        });

    Ok(json!({
        "ok": true,
        "data": data
    }))
}

async fn fetch_api_health(config: &Config) -> Result<Value, CliError> {
    let base_url = config.api_url.trim_end_matches('/');
    let response = reqwest::Client::new()
        .get(format!("{base_url}/api/v1/health"))
        .send()
        .await
        .map_err(|error| CliError::new(error.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| CliError::new(error.to_string()))?;
    let body = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }));

    Ok(json!({
        "reachable": status.is_success(),
        "status_code": status.as_u16(),
        "body": body,
    }))
}

fn upgrade_response(
    checkout_dir: Option<&str>,
    profile: memorynexus::install::InstallProfile,
    apply: bool,
    pull: bool,
    rebuild_mcp: bool,
    rebuild_api: bool,
    skip_tests: bool,
) -> Result<Value, CliError> {
    let checkout = checkout_dir
        .map(|path| resolve_checkout_dir(Some(path)))
        .transpose()?;
    let plan = upgrade_plan_value(profile, pull, rebuild_mcp, !skip_tests, rebuild_api);
    if !apply {
        return Ok(json!({
            "ok": true,
            "data": {
                "mode": "plan",
                "profile": profile.as_str(),
                "checkout": checkout
                    .as_ref()
                    .map(|path| path.display().to_string()),
                "plan": plan,
                "apply_hint": "rerun with --apply to execute these local commands",
            }
        }));
    }

    if profile != memorynexus::install::InstallProfile::Developer {
        return Err(CliError::new(
            "apply=true currently executes only Developer Profile source-build steps; use the binary-first plan commands for this profile or choose --profile developer explicitly",
        ));
    }

    let checkout = checkout.ok_or_else(|| {
        CliError::new("--checkout is required when applying Developer Profile source-build steps")
    })?;

    let dirty = git_status_short(&checkout)?;
    if pull && !dirty.trim().is_empty() {
        return Err(CliError::new(
            "refusing to git pull with local changes; commit/stash them or rerun without --pull",
        ));
    }

    let mut results = Vec::new();
    if pull {
        results.push(run_local_command(&checkout, "git", &["pull"])?);
    }
    if !skip_tests {
        results.push(run_local_command(&checkout, "cargo", &["test"])?);
    }
    if rebuild_mcp {
        results.push(run_local_command(
            &checkout,
            "cargo",
            &["build", "--bin", "memorynexus-mcp"],
        )?);
    }
    if rebuild_api {
        results.push(run_local_command(
            &checkout,
            "cargo",
            &["build", "--bin", "memorynexus"],
        )?);
    }

    Ok(json!({
        "ok": true,
        "data": {
            "mode": "applied",
            "profile": profile.as_str(),
            "checkout": checkout.display().to_string(),
            "plan": plan,
            "results": results,
            "restart_required": {
                "api": rebuild_api,
                "mcp_client": true,
                "note": "Restart the API after backend changes and reload the agent MCP client so it starts a fresh memorynexus-mcp process."
            }
        }
    }))
}

fn resolve_checkout_dir(checkout_dir: Option<&str>) -> Result<std::path::PathBuf, CliError> {
    let path = checkout_dir
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .map_err(|error| CliError::new(error.to_string()))?;
    if !path.join("Cargo.toml").exists() {
        return Err(CliError::new(format!(
            "{} does not look like the MemoryNexus checkout",
            path.display()
        )));
    }
    Ok(path)
}

fn checkout_status(checkout: &std::path::Path) -> Value {
    let git_head = run_local_command(checkout, "git", &["log", "-1", "--oneline"]).ok();
    let git_status = git_status_short(checkout).ok();

    json!({
        "path": checkout.display().to_string(),
        "git_head": git_head.and_then(|result| result.get("stdout").cloned()),
        "dirty": git_status
            .as_deref()
            .map(|status| !status.trim().is_empty())
            .unwrap_or(false),
        "git_status_short": git_status,
    })
}

fn git_status_short(checkout: &std::path::Path) -> Result<String, CliError> {
    let output = std::process::Command::new("git")
        .arg("status")
        .arg("--short")
        .current_dir(checkout)
        .output()
        .map_err(|error| CliError::new(error.to_string()))?;
    if !output.status.success() {
        return Err(CliError::new(String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_local_command(
    checkout: &std::path::Path,
    program: &str,
    args: &[&str],
) -> Result<Value, CliError> {
    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(checkout)
        .output()
        .map_err(|error| CliError::new(error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(CliError::new(format!(
            "{} {} failed: {}",
            program,
            args.join(" "),
            stderr
        )));
    }

    Ok(json!({
        "command": std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" "),
        "stdout": stdout,
        "stderr": stderr,
    }))
}

fn upgrade_plan_value(
    profile: memorynexus::install::InstallProfile,
    pull: bool,
    rebuild_mcp: bool,
    run_tests: bool,
    rebuild_api: bool,
) -> Value {
    memorynexus::install::install_plan_value(
        profile,
        memorynexus::install::InstallPlanOptions::new(
            DEFAULT_API_URL,
            None,
            None,
            memorynexus::install::ReleaseTarget::detect(),
        )
        .with_source_flags(pull, rebuild_mcp, run_tests, rebuild_api),
    )
}

fn require_token(config: &Config) -> Result<String, CliError> {
    config
        .token
        .clone()
        .ok_or_else(|| CliError::new("MEMORYNEXUS_TOKEN is required"))
}

fn required_flag(args: &[String], flag: &str) -> Result<String, CliError> {
    optional_flag(args, flag).ok_or_else(|| CliError::new(format!("{flag} is required")))
}

fn optional_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .filter(|value| !value.starts_with("--"))
        .cloned()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn parse_usize_flag(args: &[String], flag: &str, default: usize) -> Result<usize, CliError> {
    optional_flag(args, flag)
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| CliError::new(format!("{flag} must be a positive integer")))
        })
        .unwrap_or(Ok(default))
}

fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn with_query(base_url: &str, pairs: &[(&str, &str)]) -> Result<String, CliError> {
    let mut url =
        reqwest::Url::parse(base_url).map_err(|error| CliError::new(error.to_string()))?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

fn render_output(value: &Value, output_format: OutputFormat) -> String {
    match output_format {
        OutputFormat::Json => value.to_string(),
        OutputFormat::Human => render_human_output(value),
    }
}

fn render_human_output(value: &Value) -> String {
    if value.get("ok") == Some(&Value::Bool(false)) {
        return value["error"]["message"]
            .as_str()
            .unwrap_or("command failed")
            .to_string();
    }

    let data = value.get("data").unwrap_or(value);
    if let Some(script) = data.get("script").and_then(Value::as_str) {
        return script.to_string();
    }

    let mut lines = vec!["MemoryNexus".to_string()];
    if let Some(user) = data.get("user") {
        if let Some(email) = user.get("email").and_then(Value::as_str) {
            lines.push(format!("User: {email}"));
        }
    }
    if let Some(token) = data.get("token").and_then(Value::as_str) {
        lines.push(format!("Token: {}", redact_secret(token)));
    }
    if let Some(items) = data.get("items").and_then(Value::as_array) {
        if items.is_empty() {
            lines.push("No items.".to_string());
        } else {
            for item in items {
                lines.push(render_human_item(item));
            }
        }
    } else if let Some(id) = data.get("id").and_then(Value::as_str) {
        lines.push(render_human_item(data));
        lines.push(format!("ID: {id}"));
    } else if lines.len() == 1 {
        lines.push(serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string()));
    }

    lines.join("\n")
}

fn render_human_item(item: &Value) -> String {
    let title = item
        .get("title")
        .or_else(|| item.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Untitled");
    let id = item.get("id").and_then(Value::as_str).unwrap_or("-");
    let mut line = format!("- {title} ({id})");

    if let Some(created_at) = item.get("created_at").and_then(Value::as_str) {
        line.push_str(&format!(" {created_at}"));
    }
    if let Some(tags) = item.get("tags").and_then(Value::as_array) {
        let tags = tags
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !tags.is_empty() {
            line.push_str(&format!(" [{tags}]"));
        }
    }
    if let Some(content) = item.get("content").and_then(Value::as_str) {
        line.push_str(&format!("\n  {}", truncate(content, 96)));
    }

    line
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

fn redact_secret(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "[redacted]".to_string();
    }

    let prefix = chars.iter().take(4).collect::<String>();
    let suffix = chars[chars.len() - 4..].iter().collect::<String>();
    format!("{prefix}...{suffix}")
}

fn completion_script(shell: Shell) -> String {
    match shell {
        Shell::Bash => BASH_COMPLETION.to_string(),
        Shell::Zsh => ZSH_COMPLETION.to_string(),
        Shell::Fish => FISH_COMPLETION.to_string(),
    }
}

const BASH_COMPLETION: &str = r#"_memorynexus_cli()
{
  local cur prev commands
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"
  commands="health config version completion install upgrade auth space family lens memory reminder remind review search"

  case "$prev" in
    memorynexus-cli)
      COMPREPLY=( $(compgen -W "$commands --output --format" -- "$cur") )
      return 0
      ;;
    completion)
      COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
      return 0
      ;;
    --output|--format)
      COMPREPLY=( $(compgen -W "json human" -- "$cur") )
      return 0
      ;;
  esac
}
complete -F _memorynexus_cli memorynexus-cli
"#;

const ZSH_COMPLETION: &str = r#"#compdef memorynexus-cli

_memorynexus_cli() {
  local -a commands
  commands=(
    'health:Check API health'
    'config:Show runtime AI config'
    'version:Show local CLI version'
    'completion:Generate shell completion'
    'install:Inspect local install status'
    'upgrade:Plan or apply local upgrade steps'
    'auth:Register or log in'
    'space:Manage Cognitive Spaces'
    'family:Manage shared family spaces'
    'lens:Manage and run lenses'
    'memory:Create, list, inspect, or delete memories'
    'reminder:Manage reminders'
    'remind:Alias for reminder'
    'review:Create or inspect review reports'
    'search:Search memories'
  )

  _arguments \
    '--output[Set output format]:format:(json human)' \
    '--format[Set output format]:format:(json human)' \
    '1:command:->command' \
    '*::arg:->args'

  case $state in
    command)
      _describe 'command' commands
      ;;
    args)
      case ${words[2]} in
        completion)
          _values 'shell' bash zsh fish
          ;;
      esac
      ;;
  esac
}

_memorynexus_cli "$@"
"#;

const FISH_COMPLETION: &str = r#"complete -c memorynexus-cli -f
complete -c memorynexus-cli -n '__fish_use_subcommand' -a 'health config version completion install upgrade auth space family lens memory reminder remind review search'
complete -c memorynexus-cli -l output -d 'Set output format' -xa 'json human'
complete -c memorynexus-cli -l format -d 'Set output format' -xa 'json human'
complete -c memorynexus-cli -n '__fish_seen_subcommand_from completion' -a 'bash zsh fish'
"#;

fn usage() -> &'static str {
    "usage: memorynexus-cli [--output json|human] <health|config|version|completion|install|upgrade|auth|space|family|lens|memory|reminder|remind|review|search> ..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_output_format_without_changing_default_json() {
        let invocation = parse_cli(["memorynexus-cli", "--output", "human", "space", "list"])
            .expect("parse human output");
        assert_eq!(invocation.output_format, OutputFormat::Human);
        assert_eq!(invocation.command, Command::SpaceList);

        let default_invocation =
            parse_cli(["memorynexus-cli", "space", "list"]).expect("parse default output");
        assert_eq!(default_invocation.output_format, OutputFormat::Json);
        assert_eq!(default_invocation.command, Command::SpaceList);
    }

    #[test]
    fn parses_format_alias_as_leading_global_flag() {
        let invocation = parse_cli([
            "memorynexus-cli",
            "--format",
            "human",
            "memory",
            "list",
            "--space",
            "space-123",
        ])
        .expect("parse format alias");

        assert_eq!(invocation.output_format, OutputFormat::Human);
        assert_eq!(
            invocation.command,
            Command::MemoryList {
                space_id: Some("space-123".to_string()),
                limit: 20,
                offset: 0,
            }
        );
    }

    #[test]
    fn rejects_format_alias_after_command_arguments() {
        let error = parse_cli([
            "memorynexus-cli",
            "memory",
            "list",
            "--space",
            "space-123",
            "--format",
            "human",
        ])
        .unwrap_err();

        assert_eq!(error.to_string(), "--format must be a leading global flag");
    }

    #[test]
    fn rejects_unknown_output_format() {
        let error = parse_cli(["memorynexus-cli", "--output", "yaml", "health"]).unwrap_err();
        assert_eq!(error.to_string(), "--output must be json or human");
    }

    #[test]
    fn parses_completion_command() {
        let command = parse_command(["memorynexus-cli", "completion", "zsh"]).unwrap();
        assert_eq!(command, Command::Completion { shell: Shell::Zsh });
    }

    #[test]
    fn completion_defaults_to_human_script_output() {
        let invocation =
            parse_cli(["memorynexus-cli", "completion", "bash"]).expect("parse completion");
        assert_eq!(invocation.output_format, OutputFormat::Human);
        assert_eq!(
            invocation.command,
            Command::Completion { shell: Shell::Bash }
        );

        let json_invocation =
            parse_cli(["memorynexus-cli", "--output", "json", "completion", "bash"])
                .expect("parse json completion");
        assert_eq!(json_invocation.output_format, OutputFormat::Json);
    }

    #[test]
    fn renders_human_memory_list_with_titles_and_ids() {
        let value = json!({
            "ok": true,
            "data": {
                "items": [
                    {
                        "id": "mem-1",
                        "title": "Rust practice",
                        "content": "today I practiced Rust ownership and lifetimes",
                        "tags": ["rust", "learning"],
                        "created_at": "2026-05-26T08:00:00Z"
                    }
                ]
            }
        });

        let rendered = render_output(&value, OutputFormat::Human);

        assert!(rendered.contains("MemoryNexus"));
        assert!(rendered.contains("Rust practice"));
        assert!(rendered.contains("mem-1"));
        assert!(rendered.contains("rust, learning"));
    }

    #[test]
    fn renders_human_auth_token_export_hint() {
        let value = json!({
            "ok": true,
            "data": {
                "token": "jwt-token",
                "user": {
                    "email": "alice@example.com"
                }
            }
        });

        let rendered = render_output(&value, OutputFormat::Human);

        assert!(rendered.contains("alice@example.com"));
        assert!(rendered.contains("Token: jwt-...oken"));
        assert!(!rendered.contains("export MEMORYNEXUS_TOKEN"));
        assert!(!rendered.contains("jwt-token"));
    }

    #[test]
    fn generates_zsh_completion_script_with_common_commands() {
        let script = completion_script(Shell::Zsh);

        assert!(script.contains("#compdef memorynexus-cli"));
        assert!(script.contains("completion:Generate shell completion"));
        assert!(script.contains("--output"));
        assert!(script.contains("memory"));
        assert!(script.contains("lens"));
    }

    #[test]
    fn parses_health_command() {
        let command = parse_command(["memorynexus-cli", "health"]).unwrap();
        assert_eq!(command, Command::Health);
    }

    #[test]
    fn parses_config_command() {
        let command = parse_command(["memorynexus-cli", "config"]).unwrap();
        assert_eq!(command, Command::Config);
    }

    #[test]
    fn parses_version_command() {
        let command = parse_command(["memorynexus-cli", "version"]).unwrap();
        assert_eq!(command, Command::Version);
    }

    #[test]
    fn parses_install_status_command() {
        let command = parse_command([
            "memorynexus-cli",
            "install",
            "status",
            "--checkout",
            "/tmp/MemoryNexus",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::InstallStatus {
                checkout_dir: Some("/tmp/MemoryNexus".to_string()),
                profile: None,
            }
        );
    }

    #[test]
    fn parses_install_status_profile_flag() {
        let command =
            parse_command(["memorynexus-cli", "install", "status", "--profile", "trial"]).unwrap();

        assert_eq!(
            command,
            Command::InstallStatus {
                checkout_dir: None,
                profile: Some(memorynexus::install::InstallProfile::Trial),
            }
        );
    }

    #[test]
    fn parses_upgrade_plan_and_apply_flags() {
        let command = parse_command([
            "memorynexus-cli",
            "upgrade",
            "--checkout",
            "/tmp/MemoryNexus",
            "--profile",
            "developer",
            "--pull",
            "--rebuild-mcp",
            "--rebuild-api",
            "--skip-tests",
            "--apply",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Upgrade {
                checkout_dir: Some("/tmp/MemoryNexus".to_string()),
                profile: memorynexus::install::InstallProfile::Developer,
                apply: true,
                pull: true,
                rebuild_mcp: true,
                rebuild_api: true,
                skip_tests: true,
            }
        );
    }

    #[test]
    fn install_status_reports_all_profiles_and_release_target() {
        let status =
            memorynexus::install::install_status_value(memorynexus::install::InstallStatusInput {
                selected_profile: Some(memorynexus::install::InstallProfile::Trial),
                api_url: "https://demo.memorynexus.example".to_string(),
                api_health: json!({"reachable": true}),
                local: json!({"version": "0.1.0", "binary": "/tmp/memorynexus-cli"}),
                checkout: None,
                release_tag: Some("v0.1.0".to_string()),
                bin_dir: Some("/tmp/memorynexus/bin".to_string()),
                binary_path: Some("/tmp/memorynexus/bin/memorynexus-mcp".to_string()),
                target: memorynexus::install::ReleaseTarget::supported_for_test(
                    "macos",
                    "arm64",
                    "aarch64-apple-darwin",
                ),
            });

        assert_eq!(status["selected_profile"], "trial");
        assert_eq!(status["recommended_profile"], "trial");
        assert_eq!(status["release"]["target"], "aarch64-apple-darwin");
        assert_eq!(
            status["binary"]["path"],
            "/tmp/memorynexus/bin/memorynexus-mcp"
        );
        assert!(status["profiles"]["trial"].is_object());
        assert!(status["profiles"]["local-one-click"].is_object());
        assert!(status["profiles"]["production"].is_object());
        assert!(status["profiles"]["developer"].is_object());
        assert_eq!(status["fallback"]["source_build_required"], false);
    }

    #[test]
    fn trial_profile_plan_avoids_source_and_local_services() {
        let plan = memorynexus::install::install_plan_value(
            memorynexus::install::InstallProfile::Trial,
            memorynexus::install::InstallPlanOptions::for_test("v0.1.0", "aarch64-apple-darwin"),
        );
        let text = plan.to_string();

        assert!(text.contains("memorynexus-mcp"));
        assert!(text.contains("tools/list"));
        assert!(!text.contains("cargo"));
        assert!(!text.contains("Docker"));
        assert!(!text.contains("PostgreSQL"));
        assert!(!text.contains("Qdrant"));
    }

    #[test]
    fn local_one_click_profile_plan_uses_release_archive_not_cargo() {
        let plan = memorynexus::install::install_plan_value(
            memorynexus::install::InstallProfile::LocalOneClick,
            memorynexus::install::InstallPlanOptions::for_test(
                "v0.1.0",
                "x86_64-unknown-linux-gnu",
            ),
        );
        let text = plan.to_string();

        assert!(text.contains("memorynexus-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"));
        assert!(text.contains("sha256"));
        assert!(text.contains("docker compose up -d postgres qdrant"));
        assert!(text.contains("/api/v1/health"));
        assert!(text.contains("tools/list"));
        assert!(!text.contains("cargo"));
        assert!(!text.contains("rustup"));
        assert!(!text.contains("rustc"));
    }

    #[test]
    fn developer_profile_keeps_source_build_path() {
        let plan = memorynexus::install::install_plan_value(
            memorynexus::install::InstallProfile::Developer,
            memorynexus::install::InstallPlanOptions::for_test("v0.1.0", "aarch64-apple-darwin"),
        );
        let text = plan.to_string();

        assert!(text.contains("cargo test"));
        assert!(text.contains("cargo build --bin memorynexus-mcp"));
    }

    #[test]
    fn unsupported_target_reports_source_fallback_reason() {
        let status =
            memorynexus::install::install_status_value(memorynexus::install::InstallStatusInput {
                selected_profile: Some(memorynexus::install::InstallProfile::Trial),
                api_url: "http://localhost:8080".to_string(),
                api_health: json!({"reachable": false}),
                local: json!({}),
                checkout: None,
                release_tag: Some("v0.1.0".to_string()),
                bin_dir: None,
                binary_path: None,
                target: memorynexus::install::ReleaseTarget::unsupported_for_test("linux", "arm64"),
            });

        assert_eq!(status["binary"]["available"], false);
        assert_eq!(status["fallback"]["source_build_required"], true);
        assert!(status["fallback"]["reason"]
            .as_str()
            .unwrap()
            .contains("unsupported OS/arch"));
    }

    #[test]
    fn developer_upgrade_plan_defaults_to_tests_and_restart_without_apply() {
        let plan = upgrade_plan_value(
            memorynexus::install::InstallProfile::Developer,
            false,
            false,
            true,
            false,
        );
        let steps = plan["steps"].as_array().unwrap();

        assert_eq!(steps[0]["command"], "git status --short");
        assert!(steps.iter().any(|step| step["command"] == "cargo test"));
        assert!(steps
            .iter()
            .any(|step| step["command"] == "restart API and reload MCP client"));
        assert!(!steps.iter().any(|step| step["command"] == "git pull"));
    }

    #[test]
    fn upgrade_plan_includes_binary_rebuild_steps_when_requested() {
        let plan = upgrade_plan_value(
            memorynexus::install::InstallProfile::Developer,
            true,
            true,
            true,
            true,
        );
        let commands: Vec<&str> = plan["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| step["command"].as_str().unwrap())
            .collect();

        assert!(commands.contains(&"git pull"));
        assert!(commands.contains(&"cargo build --bin memorynexus-mcp"));
        assert!(commands.contains(&"cargo build --bin memorynexus"));
    }

    #[test]
    fn parses_auth_login_command() {
        let command = parse_command([
            "memorynexus-cli",
            "auth",
            "login",
            "--email",
            "alice@example.com",
            "--password",
            "secret123",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::AuthLogin {
                email: "alice@example.com".to_string(),
                password: "secret123".to_string(),
            }
        );
    }

    #[test]
    fn parses_auth_register_command_with_name_alias() {
        let command = parse_command([
            "memorynexus-cli",
            "auth",
            "register",
            "--email",
            "alice@example.com",
            "--name",
            "Alice",
            "--password",
            "secret123",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::AuthRegister {
                email: "alice@example.com".to_string(),
                username: "Alice".to_string(),
                password: "secret123".to_string(),
            }
        );
    }

    #[test]
    fn parses_memory_add_command_with_tags() {
        let command = parse_command([
            "memorynexus-cli",
            "memory",
            "add",
            "--content",
            "today I practiced Rust",
            "--title",
            "learning",
            "--tags",
            "rust,learning",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::MemoryAdd {
                space_id: None,
                content: "today I practiced Rust".to_string(),
                title: Some("learning".to_string()),
                tags: vec!["rust".to_string(), "learning".to_string()],
                memory_type: "text".to_string(),
                is_shared: false,
            }
        );
    }

    #[test]
    fn parses_space_create_and_list_commands() {
        let create = parse_command([
            "memorynexus-cli",
            "space",
            "create",
            "--name",
            "Personal Space",
            "--description",
            "Private cognitive space",
        ])
        .unwrap();
        let list = parse_command(["memorynexus-cli", "space", "list"]).unwrap();

        assert_eq!(
            create,
            Command::SpaceCreate {
                name: "Personal Space".to_string(),
                description: Some("Private cognitive space".to_string()),
            }
        );
        assert_eq!(list, Command::SpaceList);
    }

    #[test]
    fn parses_family_commands() {
        let create = parse_command([
            "memorynexus-cli",
            "family",
            "create",
            "--name",
            "Family Space",
            "--description",
            "Shared family cognition",
        ])
        .unwrap();
        let list = parse_command(["memorynexus-cli", "family", "list"]).unwrap();
        let members = parse_command([
            "memorynexus-cli",
            "family",
            "members",
            "--space",
            "space-123",
        ])
        .unwrap();
        let invite = parse_command([
            "memorynexus-cli",
            "family",
            "invite",
            "--space",
            "space-123",
            "--role",
            "editor",
            "--expires-in-days",
            "7",
        ])
        .unwrap();
        let accept = parse_command([
            "memorynexus-cli",
            "family",
            "accept",
            "--code",
            "invite-code",
        ])
        .unwrap();
        let role = parse_command([
            "memorynexus-cli",
            "family",
            "role",
            "--space",
            "space-123",
            "--user",
            "user-123",
            "--role",
            "viewer",
        ])
        .unwrap();

        assert_eq!(
            create,
            Command::FamilyCreate {
                name: "Family Space".to_string(),
                description: Some("Shared family cognition".to_string()),
            }
        );
        assert_eq!(list, Command::FamilyList);
        assert_eq!(
            members,
            Command::FamilyMembers {
                space_id: "space-123".to_string(),
            }
        );
        assert_eq!(
            invite,
            Command::FamilyInvite {
                space_id: "space-123".to_string(),
                role: "editor".to_string(),
                expires_in_days: Some(7),
            }
        );
        assert_eq!(
            accept,
            Command::FamilyAccept {
                code: "invite-code".to_string(),
            }
        );
        assert_eq!(
            role,
            Command::FamilyRole {
                space_id: "space-123".to_string(),
                user_id: "user-123".to_string(),
                role: "viewer".to_string(),
            }
        );
    }

    #[test]
    fn parses_lens_create_list_and_get_commands() {
        let create = parse_command([
            "memorynexus-cli",
            "lens",
            "create",
            "--space",
            "space-123",
            "--name",
            "Project Context",
            "--description",
            "Interpret project memory",
            "--strategy",
            "project_context",
            "--output",
            "brief",
            "--retrieval",
            "semantic",
        ])
        .unwrap();
        let list =
            parse_command(["memorynexus-cli", "lens", "list", "--space", "space-123"]).unwrap();
        let get = parse_command(["memorynexus-cli", "lens", "get", "lens-123"]).unwrap();

        assert_eq!(
            create,
            Command::LensCreate {
                space_id: "space-123".to_string(),
                name: "Project Context".to_string(),
                description: Some("Interpret project memory".to_string()),
                strategy: "project_context".to_string(),
                output_format: "brief".to_string(),
                retrieval_mode: "semantic".to_string(),
            }
        );
        assert_eq!(
            list,
            Command::LensList {
                space_id: "space-123".to_string(),
            }
        );
        assert_eq!(
            get,
            Command::LensGet {
                id: "lens-123".to_string(),
            }
        );
    }

    #[test]
    fn parses_lens_templates_and_template_create_commands() {
        let templates = parse_command(["memorynexus-cli", "lens", "templates"]).unwrap();
        let create = parse_command([
            "memorynexus-cli",
            "lens",
            "create",
            "--space",
            "space-123",
            "--template",
            "project_context",
        ])
        .unwrap();

        assert_eq!(templates, Command::LensTemplates);
        assert_eq!(
            create,
            Command::LensCreate {
                space_id: "space-123".to_string(),
                name: "Project Context".to_string(),
                description: Some(
                    "Interpret project memories for planning and direction.".to_string()
                ),
                strategy: "project_context".to_string(),
                output_format: "brief".to_string(),
                retrieval_mode: "semantic".to_string(),
            }
        );
    }

    #[test]
    fn parses_lens_run_and_run_get_commands() {
        let run = parse_command([
            "memorynexus-cli",
            "lens",
            "run",
            "lens-123",
            "--query",
            "Summarize the current project direction",
            "--limit",
            "3",
        ])
        .unwrap();
        let get = parse_command(["memorynexus-cli", "lens", "run", "get", "run-123"]).unwrap();

        assert_eq!(
            run,
            Command::LensRun {
                lens_id: "lens-123".to_string(),
                query: "Summarize the current project direction".to_string(),
                limit: 3,
            }
        );
        assert_eq!(
            get,
            Command::LensRunGet {
                id: "run-123".to_string(),
            }
        );
    }

    #[test]
    fn parses_lens_run_list_command() {
        let by_lens = parse_command([
            "memorynexus-cli",
            "lens",
            "run",
            "list",
            "--lens",
            "lens-123",
            "--limit",
            "3",
        ])
        .unwrap();
        let by_space = parse_command([
            "memorynexus-cli",
            "lens",
            "run",
            "list",
            "--space",
            "space-123",
        ])
        .unwrap();

        assert_eq!(
            by_lens,
            Command::LensRunList {
                lens_id: Some("lens-123".to_string()),
                space_id: None,
                limit: 3,
            }
        );
        assert_eq!(
            by_space,
            Command::LensRunList {
                lens_id: None,
                space_id: Some("space-123".to_string()),
                limit: 20,
            }
        );
    }

    #[test]
    fn parses_memory_add_with_space_id() {
        let command = parse_command([
            "memorynexus-cli",
            "memory",
            "add",
            "--space",
            "space-123",
            "--content",
            "today I practiced Rust",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::MemoryAdd {
                space_id: Some("space-123".to_string()),
                content: "today I practiced Rust".to_string(),
                title: None,
                tags: vec![],
                memory_type: "text".to_string(),
                is_shared: false,
            }
        );
    }

    #[test]
    fn parses_memory_get_and_delete_commands() {
        let get = parse_command(["memorynexus-cli", "memory", "get", "mem-123"]).unwrap();
        let delete = parse_command(["memorynexus-cli", "memory", "delete", "mem-123"]).unwrap();

        assert_eq!(
            get,
            Command::MemoryGet {
                id: "mem-123".to_string(),
            }
        );
        assert_eq!(
            delete,
            Command::MemoryDelete {
                id: "mem-123".to_string(),
            }
        );
    }

    #[test]
    fn parses_reminder_commands() {
        let add = parse_command([
            "memorynexus-cli",
            "reminder",
            "add",
            "--space",
            "space-123",
            "--content",
            "Review Rust practice",
            "--at",
            "2026-05-26T09:00:00Z",
            "--repeat",
            "weekly",
            "--channel",
            "in_app",
        ])
        .unwrap();
        let list = parse_command([
            "memorynexus-cli",
            "reminder",
            "list",
            "--space",
            "space-123",
            "--due",
            "--limit",
            "5",
        ])
        .unwrap();
        let complete =
            parse_command(["memorynexus-cli", "reminder", "complete", "reminder-123"]).unwrap();
        let failed_delivery = parse_command([
            "memorynexus-cli",
            "reminder",
            "delivery",
            "reminder-123",
            "--status",
            "failed",
            "--error",
            "client notification panel unavailable",
        ])
        .unwrap();

        assert_eq!(
            add,
            Command::ReminderAdd {
                space_id: "space-123".to_string(),
                content: "Review Rust practice".to_string(),
                remind_at: "2026-05-26T09:00:00Z".to_string(),
                title: None,
                memory_id: None,
                repeat_rule: Some("weekly".to_string()),
                delivery_channel: Some("in_app".to_string()),
            }
        );
        assert_eq!(
            list,
            Command::ReminderList {
                space_id: "space-123".to_string(),
                due_only: true,
                include_completed: false,
                limit: 5,
            }
        );
        assert_eq!(
            complete,
            Command::ReminderComplete {
                id: "reminder-123".to_string(),
            }
        );
        assert_eq!(
            failed_delivery,
            Command::ReminderDelivery {
                id: "reminder-123".to_string(),
                status: "failed".to_string(),
                error: Some("client notification panel unavailable".to_string()),
            }
        );
    }

    #[test]
    fn parses_remind_alias() {
        let command =
            parse_command(["memorynexus-cli", "remind", "list", "--space", "space-123"]).unwrap();

        assert_eq!(
            command,
            Command::ReminderList {
                space_id: "space-123".to_string(),
                due_only: false,
                include_completed: false,
                limit: 20,
            }
        );
    }

    #[test]
    fn parses_review_commands() {
        let create = parse_command([
            "memorynexus-cli",
            "review",
            "create",
            "--space",
            "space-123",
            "--lens",
            "lens-123",
            "--from",
            "2026-05-18T00:00:00Z",
            "--to",
            "2026-05-25T00:00:00Z",
            "--type",
            "weekly_review",
            "--limit",
            "12",
        ])
        .unwrap();
        let get = parse_command(["memorynexus-cli", "review", "get", "report-123"]).unwrap();
        let list =
            parse_command(["memorynexus-cli", "review", "list", "--space", "space-123"]).unwrap();

        assert_eq!(
            create,
            Command::ReviewCreate {
                space_id: "space-123".to_string(),
                lens_id: "lens-123".to_string(),
                window_start: "2026-05-18T00:00:00Z".to_string(),
                window_end: "2026-05-25T00:00:00Z".to_string(),
                report_type: "weekly_review".to_string(),
                limit: 12,
            }
        );
        assert_eq!(
            get,
            Command::ReviewGet {
                id: "report-123".to_string(),
            }
        );
        assert_eq!(
            list,
            Command::ReviewList {
                space_id: "space-123".to_string(),
                lens_id: None,
                limit: 20,
            }
        );
    }

    #[test]
    fn parses_semantic_search_command() {
        let command = parse_command([
            "memorynexus-cli",
            "search",
            "Rust cognitive memory",
            "--semantic",
            "--limit",
            "5",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Search {
                space_id: None,
                lens_id: None,
                query: "Rust cognitive memory".to_string(),
                semantic: true,
                limit: 5,
            }
        );
    }

    #[test]
    fn parses_lens_scoped_search_command() {
        let command = parse_command([
            "memorynexus-cli",
            "search",
            "cognitive lens",
            "--lens",
            "lens-123",
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Search {
                space_id: None,
                lens_id: Some("lens-123".to_string()),
                query: "cognitive lens".to_string(),
                semantic: false,
                limit: 20,
            }
        );
    }

    #[test]
    fn authenticated_commands_require_token() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };

        let error = build_request(
            &config,
            &Command::MemoryList {
                space_id: None,
                limit: 20,
                offset: 0,
            },
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "MEMORYNEXUS_TOKEN is required");
    }

    #[test]
    fn builds_memory_add_request_with_bearer_token_and_json_body() {
        let config = Config {
            api_url: "http://localhost:8080/".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::MemoryAdd {
                space_id: Some("space-123".to_string()),
                content: "today I practiced Rust".to_string(),
                title: Some("learning".to_string()),
                tags: vec!["rust".to_string()],
                memory_type: "text".to_string(),
                is_shared: true,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/memories");
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.body,
            Some(json!({
                "title": "learning",
                "content": "today I practiced Rust",
                "memory_type": "text",
                "tags": ["rust"],
                "is_shared": true,
                "space_id": "space-123",
            }))
        );
    }

    #[test]
    fn builds_space_create_request() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::SpaceCreate {
                name: "Personal Space".to_string(),
                description: None,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/spaces");
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.body,
            Some(json!({
                "name": "Personal Space",
                "description": null,
            }))
        );
    }

    #[test]
    fn builds_family_requests() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let create = build_request(
            &config,
            &Command::FamilyCreate {
                name: "Family Space".to_string(),
                description: None,
            },
        )
        .unwrap();
        let list = build_request(&config, &Command::FamilyList).unwrap();
        let members = build_request(
            &config,
            &Command::FamilyMembers {
                space_id: "space-123".to_string(),
            },
        )
        .unwrap();
        let invite = build_request(
            &config,
            &Command::FamilyInvite {
                space_id: "space-123".to_string(),
                role: "viewer".to_string(),
                expires_in_days: Some(7),
            },
        )
        .unwrap();
        let accept = build_request(
            &config,
            &Command::FamilyAccept {
                code: "invite-code".to_string(),
            },
        )
        .unwrap();
        let role = build_request(
            &config,
            &Command::FamilyRole {
                space_id: "space-123".to_string(),
                user_id: "user-123".to_string(),
                role: "editor".to_string(),
            },
        )
        .unwrap();

        assert_eq!(create.method, HttpMethod::Post);
        assert_eq!(create.url, "http://localhost:8080/api/v1/spaces");
        assert_eq!(
            create.body,
            Some(json!({
                "name": "Family Space",
                "description": null,
                "space_type": "family",
            }))
        );
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/spaces?space_type=family"
        );
        assert_eq!(members.method, HttpMethod::Get);
        assert_eq!(
            members.url,
            "http://localhost:8080/api/v1/spaces/space-123/members"
        );
        assert_eq!(invite.method, HttpMethod::Post);
        assert_eq!(
            invite.url,
            "http://localhost:8080/api/v1/spaces/space-123/invites"
        );
        assert_eq!(
            invite.body,
            Some(json!({
                "role": "viewer",
                "expires_in_days": 7,
            }))
        );
        assert_eq!(accept.method, HttpMethod::Post);
        assert_eq!(
            accept.url,
            "http://localhost:8080/api/v1/spaces/invites/accept"
        );
        assert_eq!(role.method, HttpMethod::Patch);
        assert_eq!(
            role.url,
            "http://localhost:8080/api/v1/spaces/space-123/members/user-123"
        );
        assert_eq!(
            role.body,
            Some(json!({
                "role": "editor",
            }))
        );
    }

    #[test]
    fn builds_config_request_without_token() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };
        let request = build_request(&config, &Command::Config).unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.url, "http://localhost:8080/api/v1/ai/config");
        assert_eq!(request.token, None);
    }

    #[test]
    fn builds_lens_create_request() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::LensCreate {
                space_id: "space-123".to_string(),
                name: "Project Context".to_string(),
                description: None,
                strategy: "project_context".to_string(),
                output_format: "brief".to_string(),
                retrieval_mode: "semantic".to_string(),
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/lenses");
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.body,
            Some(json!({
                "space_id": "space-123",
                "name": "Project Context",
                "description": null,
                "strategy": "project_context",
                "output_format": "brief",
                "retrieval_mode": "semantic",
            }))
        );
    }

    #[tokio::test]
    async fn executes_lens_templates_without_token_or_api() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: None,
        };
        let output = execute(&config, &Command::LensTemplates).await.unwrap();

        assert_eq!(output["ok"], true);
        assert!(output["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|template| template["id"] == "project_context"));
        assert!(output["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|template| template["id"] == "personal_context"));
    }

    #[test]
    fn builds_lens_run_request() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::LensRun {
                lens_id: "lens-123".to_string(),
                query: "Summarize the current project direction".to_string(),
                limit: 3,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "http://localhost:8080/api/v1/lens-runs");
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.body,
            Some(json!({
                "lens_id": "lens-123",
                "query": "Summarize the current project direction",
                "limit": 3,
            }))
        );
    }

    #[test]
    fn builds_lens_run_list_request() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::LensRunList {
                lens_id: Some("lens-123".to_string()),
                space_id: None,
                limit: 3,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/lens-runs?limit=3&lens_id=lens-123"
        );
    }

    #[test]
    fn builds_reminder_requests() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let add = build_request(
            &config,
            &Command::ReminderAdd {
                space_id: "space-123".to_string(),
                content: "Review Rust practice".to_string(),
                remind_at: "2026-05-26T09:00:00Z".to_string(),
                title: Some("Review".to_string()),
                memory_id: Some("memory-123".to_string()),
                repeat_rule: Some("weekly".to_string()),
                delivery_channel: Some("in_app".to_string()),
            },
        )
        .unwrap();
        let list = build_request(
            &config,
            &Command::ReminderList {
                space_id: "space-123".to_string(),
                due_only: true,
                include_completed: false,
                limit: 5,
            },
        )
        .unwrap();
        let complete = build_request(
            &config,
            &Command::ReminderComplete {
                id: "reminder-123".to_string(),
            },
        )
        .unwrap();
        let failed_delivery = build_request(
            &config,
            &Command::ReminderDelivery {
                id: "reminder-123".to_string(),
                status: "failed".to_string(),
                error: Some("client notification panel unavailable".to_string()),
            },
        )
        .unwrap();

        assert_eq!(add.method, HttpMethod::Post);
        assert_eq!(add.url, "http://localhost:8080/api/v1/reminders");
        assert_eq!(
            add.body,
            Some(json!({
                "space_id": "space-123",
                "memory_id": "memory-123",
                "title": "Review",
                "content": "Review Rust practice",
                "remind_at": "2026-05-26T09:00:00Z",
                "repeat_rule": "weekly",
                "delivery_channel": "in_app",
            }))
        );
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/reminders?space_id=space-123&due_only=true&include_completed=false&limit=5"
        );
        assert_eq!(complete.method, HttpMethod::Post);
        assert_eq!(
            complete.url,
            "http://localhost:8080/api/v1/reminders/reminder-123/complete"
        );
        assert_eq!(failed_delivery.method, HttpMethod::Post);
        assert_eq!(
            failed_delivery.url,
            "http://localhost:8080/api/v1/reminders/reminder-123/delivery"
        );
        assert_eq!(
            failed_delivery.body,
            Some(json!({
                "status": "failed",
                "error": "client notification panel unavailable",
            }))
        );
    }

    #[test]
    fn builds_review_requests() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };

        let create = build_request(
            &config,
            &Command::ReviewCreate {
                space_id: "space-123".to_string(),
                lens_id: "lens-123".to_string(),
                window_start: "2026-05-18T00:00:00Z".to_string(),
                window_end: "2026-05-25T00:00:00Z".to_string(),
                report_type: "weekly_review".to_string(),
                limit: 12,
            },
        )
        .unwrap();
        let get = build_request(
            &config,
            &Command::ReviewGet {
                id: "report-123".to_string(),
            },
        )
        .unwrap();
        let list = build_request(
            &config,
            &Command::ReviewList {
                space_id: "space-123".to_string(),
                lens_id: Some("lens-123".to_string()),
                limit: 5,
            },
        )
        .unwrap();

        assert_eq!(create.method, HttpMethod::Post);
        assert_eq!(create.url, "http://localhost:8080/api/v1/review-reports");
        assert_eq!(
            create.body,
            Some(json!({
                "space_id": "space-123",
                "lens_id": "lens-123",
                "window_start": "2026-05-18T00:00:00Z",
                "window_end": "2026-05-25T00:00:00Z",
                "report_type": "weekly_review",
                "limit": 12,
            }))
        );
        assert_eq!(get.method, HttpMethod::Get);
        assert_eq!(
            get.url,
            "http://localhost:8080/api/v1/review-reports/report-123"
        );
        assert_eq!(list.method, HttpMethod::Get);
        assert_eq!(
            list.url,
            "http://localhost:8080/api/v1/review-reports?space_id=space-123&limit=5&lens_id=lens-123"
        );
    }

    #[test]
    fn builds_semantic_search_request_with_encoded_query() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::Search {
                space_id: Some("space-123".to_string()),
                lens_id: None,
                query: "Rust cognitive memory".to_string(),
                semantic: true,
                limit: 5,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/search?q=Rust+cognitive+memory&semantic=true&limit=5&space_id=space-123"
        );
    }

    #[test]
    fn builds_lens_scoped_search_request() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            token: Some("jwt-token".to_string()),
        };
        let request = build_request(
            &config,
            &Command::Search {
                space_id: None,
                lens_id: Some("lens-123".to_string()),
                query: "cognitive lens".to_string(),
                semantic: false,
                limit: 20,
            },
        )
        .unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.token, Some("jwt-token".to_string()));
        assert_eq!(
            request.url,
            "http://localhost:8080/api/v1/search?q=cognitive+lens&semantic=false&limit=20&lens_id=lens-123"
        );
    }
}
