use serde_json::{json, Value};

const DEFAULT_API_URL: &str = "http://localhost:8080";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Health,
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
    LensCreate {
        space_id: String,
        name: String,
        description: Option<String>,
        strategy: String,
        output_format: String,
        retrieval_mode: String,
    },
    LensList {
        space_id: String,
    },
    LensGet {
        id: String,
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
    Search {
        space_id: Option<String>,
        query: String,
        semantic: bool,
        limit: usize,
    },
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
    let command = parse_command(std::env::args());
    let result = match command {
        Ok(command) => execute(&Config::from_env(), &command).await,
        Err(error) => Err(error),
    };

    match result {
        Ok(value) => {
            println!("{}", value);
        }
        Err(error) => {
            eprintln!("{}", error.to_json());
            std::process::exit(1);
        }
    }
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
        "auth" => parse_auth_command(&args[2..]),
        "space" => parse_space_command(&args[2..]),
        "lens" => parse_lens_command(&args[2..]),
        "memory" => parse_memory_command(&args[2..]),
        "search" => parse_search_command(&args[2..]),
        _ => Err(CliError::new(usage())),
    }
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

fn parse_lens_command(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(CliError::new("lens subcommand is required"));
    };

    match subcommand {
        "create" => Ok(Command::LensCreate {
            space_id: required_flag(args, "--space")?,
            name: required_flag(args, "--name")?,
            description: optional_flag(args, "--description"),
            strategy: optional_flag(args, "--strategy").unwrap_or_else(|| "default".to_string()),
            output_format: optional_flag(args, "--output").unwrap_or_else(|| "summary".to_string()),
            retrieval_mode: optional_flag(args, "--retrieval")
                .unwrap_or_else(|| "semantic".to_string()),
        }),
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
        _ => Err(CliError::new("unknown lens subcommand")),
    }
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

fn parse_search_command(args: &[String]) -> Result<Command, CliError> {
    let query = args
        .first()
        .filter(|query| !query.starts_with("--"))
        .cloned()
        .ok_or_else(|| CliError::new("search query is required"))?;

    Ok(Command::Search {
        space_id: optional_flag(args, "--space"),
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
        Command::Search {
            space_id,
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
    let request = build_request(config, command)?;
    let client = reqwest::Client::new();
    let mut builder = match request.method {
        HttpMethod::Get => client.get(&request.url),
        HttpMethod::Post => client.post(&request.url),
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

fn usage() -> &'static str {
    "usage: memorynexus-cli <health|auth|space|lens|memory|search> ..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_health_command() {
        let command = parse_command(["memorynexus-cli", "health"]).unwrap();
        assert_eq!(command, Command::Health);
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
                query: "Rust cognitive memory".to_string(),
                semantic: true,
                limit: 5,
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
}
