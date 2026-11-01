//! Local dotenv parsing without mutating the process environment.
use project_knowledge_core::{
    contract::error::{CoreError, Issue, codes},
    project::config::{AgentConfig, LoadedProject},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct Environment {
    values: BTreeMap<String, String>,
    pub path: Option<PathBuf>,
}

impl Environment {
    /// One explicit file, otherwise cwd/.env, then the target's .env. Process values win.
    pub fn load(project: &LoadedProject, explicit: Option<&Path>) -> Result<Self, CoreError> {
        let cwd =
            std::env::current_dir().map_err(|_| invalid("", "cannot resolve working directory"))?;
        let paths = [
            cwd.join(".env"),
            project.base_dir.join(".env"),
            project.base_dir.join("../.env"),
        ];
        let path = explicit
            .map(Path::to_path_buf)
            .or_else(|| paths.into_iter().find(|p| p.is_file()));
        let mut values = BTreeMap::new();
        if let Some(path) = &path {
            let raw = std::fs::read_to_string(path)
                .map_err(|_| invalid("/envFile", "cannot read UTF-8 env file"))?;
            let iter = dotenvy::from_read_iter(raw.trim_start_matches('\u{feff}').as_bytes());
            for item in iter {
                let (key, value) = item
                    .map_err(|_| invalid("/envFile", "invalid dotenv syntax (values withheld)"))?;
                values.entry(key).or_insert(value);
            }
        }
        Ok(Self { values, path })
    }
    pub fn get(&self, name: &str) -> Option<String> {
        std::env::var(name)
            .ok()
            .or_else(|| self.values.get(name).cloned())
    }
    pub fn configuration(&self, existing: Option<&AgentConfig>) -> Result<AgentConfig, CoreError> {
        let mut value = existing
            .map(|c| serde_json::to_value(c).expect("config JSON"))
            .unwrap_or_else(|| serde_json::json!({"enabled":true,"provider":"","model":""}));
        for (env, field) in [
            ("PROJECT_KNOWLEDGE_PROVIDER", "provider"),
            ("PROJECT_KNOWLEDGE_MODEL", "model"),
            ("PROJECT_KNOWLEDGE_BASE_URL", "baseUrl"),
            ("PROJECT_KNOWLEDGE_API_KEY_ENV", "apiKeyEnv"),
        ] {
            if let Some(v) = self.get(env) {
                value[field] = v.into();
            }
        }
        if let Some(thinking) = self.get("PROJECT_KNOWLEDGE_THINKING") {
            value["thinking"] = thinking
                .parse::<bool>()
                .map_err(|_| invalid("/agent/thinking", "thinking must be true or false"))?
                .into();
        }
        serde_json::from_value(value).map_err(|_| invalid("/agent", "invalid agent configuration"))
    }
    pub fn credential(&self, config: &AgentConfig) -> Result<String, CoreError> {
        self.get(&config.api_key_env)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                invalid(
                    "/agent/apiKeyEnv",
                    "model credential is missing; set the named environment variable or .env entry",
                )
            })
    }
}

pub fn invalid(pointer: &str, message: &str) -> CoreError {
    CoreError::new(codes::MODEL_CONFIG_INVALID, message).with_issues(vec![
        Issue::new(codes::MODEL_CONFIG_INVALID, pointer).with_hint(message),
    ])
}

pub fn validate(config: &AgentConfig) -> Result<(), CoreError> {
    if !config.enabled {
        return Err(invalid("/agent/enabled", "agent is disabled"));
    }
    if !matches!(
        config.provider.as_str(),
        "openai" | "openai-compatible" | "anthropic"
    ) {
        return Err(invalid(
            "/agent/provider",
            "supported providers: openai, openai-compatible, anthropic",
        ));
    }
    if config.model.trim().is_empty() {
        return Err(invalid("/agent/model", "model ID is required"));
    }
    if config.api_key_env.is_empty()
        || !config
            .api_key_env
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    {
        return Err(invalid(
            "/agent/apiKeyEnv",
            "invalid environment variable name",
        ));
    }
    if config.provider == "openai-compatible" && config.base_url.is_none() {
        return Err(invalid(
            "/agent/baseUrl",
            "compatible providers require an explicit base URL",
        ));
    }
    if let Some(base) = &config.base_url {
        let url = url::Url::parse(base)
            .map_err(|_| invalid("/agent/baseUrl", "invalid model base URL"))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid(
                "/agent/baseUrl",
                "base URL must use HTTP(S) without credentials, query or fragment",
            ));
        }
    }
    for (field, ok) in [
        ("maxTurns", (1..=100).contains(&config.max_turns)),
        ("maxToolCalls", (1..=500).contains(&config.max_tool_calls)),
        ("maxRepairAttempts", config.max_repair_attempts <= 2),
        (
            "timeoutSeconds",
            (1..=3600).contains(&config.timeout_seconds),
        ),
        (
            "maxInputChars",
            (1000..=2_000_000).contains(&config.max_input_chars),
        ),
        (
            "maxOutputTokens",
            (1..=65536).contains(&config.max_output_tokens),
        ),
    ] {
        if !ok {
            return Err(invalid(
                &format!("/agent/{field}"),
                "budget is outside supported bounds",
            ));
        }
    }
    Ok(())
}

pub fn save(project: &LoadedProject, config: AgentConfig) -> Result<(), CoreError> {
    validate(&config)?;
    let _lock = project_knowledge_core::collect::registry::MaintenanceLock::acquire(project)?;
    let current = LoadedProject::load(&project.config_path)?;
    if current.config_hash()? != project.config_hash()? {
        return Err(CoreError::new(
            codes::ACTIVATION_CONFLICT,
            "project configuration changed",
        ));
    }
    let mut next = project.config.clone();
    next.agent = Some(config);
    let bytes = serde_json::to_vec_pretty(&next).expect("project config JSON");
    atomic_write(&project.config_path, &bytes)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("", "file has no parent"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    file.write_all(bytes).map_err(io_error)?;
    file.as_file().sync_all().map_err(io_error)?;
    file.persist(path).map_err(|e| io_error(e.error))?;
    Ok(())
}
pub fn io_error(_: std::io::Error) -> CoreError {
    CoreError::new(codes::INTERNAL_ERROR, "agent workspace I/O failed")
}
