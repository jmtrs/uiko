#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    io::Write as _,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const TRACE_SCHEMA_VERSION: u32 = 1;
const RESULT_SCHEMA_VERSION: u32 = 1;
const PROTOCOL: &str = "g0-v1";
const TOKENIZATION_ID: &str = "unicode-lexical-v1";
const TOKEN_PATTERN: &str = r"[\p{L}\p{N}_]+|[^\s]";

static TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(TOKEN_PATTERN).expect("frozen token regex must compile"));

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Arm {
    #[serde(rename = "B_FULL")]
    BFull,
    #[serde(rename = "C_UIKO")]
    CUiko,
    #[serde(rename = "R_RENDER_ONLY")]
    RRenderOnly,
    #[serde(rename = "T_AUTHORING")]
    TAuthoring,
}

impl Arm {
    fn policy_key(self) -> &'static str {
        match self {
            Self::BFull => "B_FULL",
            Self::CUiko => "C_UIKO",
            Self::RRenderOnly => "R_RENDER_ONLY",
            Self::TAuthoring => "T_AUTHORING",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEvent {
    pub schema_version: u32,
    pub sequence: u64,
    pub run_id: String,
    pub task_id: String,
    pub arm: Arm,
    #[serde(flatten)]
    pub payload: EventPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventPayload {
    RunStart {
        base_revision: String,
        started_at: String,
        started_unix_ms: u64,
        model: Box<ModelMetadata>,
        environment: Box<EnvironmentMetadata>,
    },
    RepositoryRead {
        target: String,
    },
    RepositorySearch {
        query: String,
    },
    Edit {
        path: String,
        operation: EditOperation,
        before_text: Option<String>,
        after_text: Option<String>,
    },
    Validation {
        command: String,
        success: bool,
        #[serde(default)]
        evidence: Option<String>,
    },
    Browser {
        action: String,
        success: bool,
        #[serde(default)]
        evidence: Option<String>,
    },
    Repair {
        iteration: u32,
        category: RepairCategory,
        reason: String,
        evidence: String,
        #[serde(default)]
        source_paths: Vec<String>,
    },
    Acceptance {
        criterion_index: u32,
        passed: bool,
        evidence: String,
    },
    ProviderTokens {
        input_tokens: u64,
        output_tokens: u64,
    },
    ProtocolDeviation {
        code: String,
        description: String,
        impact: DeviationImpact,
    },
    RunEnd {
        ended_at: String,
        ended_unix_ms: u64,
        outcome: Outcome,
        #[serde(default)]
        final_revision: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EditOperation {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RepairCategory {
    #[serde(rename = "LOCAL")]
    Local,
    #[serde(rename = "WIRING")]
    Wiring,
    #[serde(rename = "COHERENCE")]
    Coherence,
    #[serde(rename = "VISUAL_FIT")]
    VisualFit,
    #[serde(rename = "ENVIRONMENT")]
    Environment,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Accepted,
    Incomplete,
    Invalidated,
    Aborted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviationImpact {
    None,
    Minor,
    Material,
    Invalidating,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMetadata {
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub agent_harness: String,
    #[serde(default)]
    pub parameters: Map<String, Value>,
    #[serde(default)]
    pub seed: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentMetadata {
    pub os: String,
    pub arch: String,
    #[serde(default)]
    pub node: Option<String>,
    #[serde(default)]
    pub rustc: Option<String>,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub schema_version: u32,
    pub protocol: &'static str,
    pub run_id: String,
    pub task_id: String,
    pub arm: Arm,
    pub base_revision: String,
    pub final_revision: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub model: ModelMetadata,
    pub environment: EnvironmentMetadata,
    pub outcome: Outcome,
    pub metrics: Metrics,
    pub repairs: Vec<RepairResult>,
    pub acceptance: Vec<AcceptanceResult>,
    pub edit_trace: Vec<EditTraceResult>,
    pub protocol_deviations: Vec<ProtocolDeviationResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub accepted_change_tokens: u64,
    pub cumulative_authored_edit_tokens: u64,
    pub non_application_edit_tokens: u64,
    pub files_touched: Vec<String>,
    pub changed_bytes: u64,
    pub changed_lines: u64,
    pub repository_read_operations: u64,
    pub repository_search_operations: u64,
    pub edit_operations: u64,
    pub validation_invocations: u64,
    pub browser_invocations: u64,
    pub acceptance_loop_actions: u64,
    pub provider_input_tokens: Option<u64>,
    pub provider_output_tokens: Option<u64>,
    pub wall_clock_ms: u64,
    pub core_modification: bool,
    pub generated_glue_touched: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    pub iteration: u32,
    pub category: RepairCategory,
    pub reason: String,
    pub evidence: String,
    pub source_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceResult {
    pub criterion_index: u32,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTraceResult {
    pub sequence: u64,
    pub path: String,
    pub path_class: PathClass,
    pub operation: EditOperation,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
    pub inserted_tokens: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolDeviationResult {
    pub code: String,
    pub description: String,
    pub impact: DeviationImpact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathClass {
    Application,
    Generated,
    Core,
    Harness,
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathPolicy {
    tokenization: TokenizationPolicy,
    arms: BTreeMap<String, ArmPathPolicy>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenizationPolicy {
    id: String,
    pattern: String,
    diff_algorithm: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArmPathPolicy {
    application_owned: Vec<String>,
    generated: Vec<String>,
    harness_owned_read_only: Vec<String>,
    core_owned: Vec<String>,
}

struct PathClassifier {
    root: PathBuf,
    application: Gitignore,
    generated: Gitignore,
    harness: Gitignore,
    core: Gitignore,
}

impl PathClassifier {
    fn from_policy(repo_root: &Path, policy: &PathPolicy, arm: Arm) -> Result<Self, String> {
        if policy.tokenization.id != TOKENIZATION_ID
            || policy.tokenization.pattern != TOKEN_PATTERN
            || policy.tokenization.diff_algorithm != "Myers diff over token sequences"
        {
            return Err("path policy tokenization no longer matches frozen g0-v1".into());
        }

        let arm_policy = policy
            .arms
            .get(arm.policy_key())
            .ok_or_else(|| format!("path policy is missing arm {}", arm.policy_key()))?;

        Ok(Self {
            root: repo_root.to_path_buf(),
            application: build_matcher(repo_root, &arm_policy.application_owned)?,
            generated: build_matcher(repo_root, &arm_policy.generated)?,
            harness: build_matcher(repo_root, &arm_policy.harness_owned_read_only)?,
            core: build_matcher(repo_root, &arm_policy.core_owned)?,
        })
    }

    fn classify(&self, relative: &str) -> PathClass {
        let path = self.root.join(relative);
        if self
            .harness
            .matched_path_or_any_parents(&path, false)
            .is_ignore()
        {
            PathClass::Harness
        } else if self
            .generated
            .matched_path_or_any_parents(&path, false)
            .is_ignore()
        {
            PathClass::Generated
        } else if self
            .core
            .matched_path_or_any_parents(&path, false)
            .is_ignore()
        {
            PathClass::Core
        } else if self
            .application
            .matched_path_or_any_parents(&path, false)
            .is_ignore()
        {
            PathClass::Application
        } else {
            PathClass::Other
        }
    }
}

fn build_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore, String> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|error| format!("invalid frozen path glob `{pattern}`: {error}"))?;
    }
    builder
        .build()
        .map_err(|error| format!("cannot build frozen path matcher: {error}"))
}

/// Append one validated event as a compact NDJSON line.
///
/// # Errors
///
/// Returns an error when the event is invalid, does not follow the existing
/// sequence, or cannot be appended.
pub fn append_event(trace_path: &Path, event_json: &str) -> Result<(), String> {
    let event: TraceEvent = serde_json::from_str(event_json)
        .map_err(|error| format!("invalid trace event: {error}"))?;
    validate_event_shape(&event)?;

    let previous = if trace_path.exists() {
        let text = fs::read_to_string(trace_path)
            .map_err(|error| format!("cannot read trace {}: {error}", trace_path.display()))?;
        text.lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(serde_json::from_str::<TraceEvent>)
            .transpose()
            .map_err(|error| format!("existing trace contains invalid JSON: {error}"))?
    } else {
        None
    };

    match previous {
        None if event.sequence != 1 => {
            return Err("the first trace event must have sequence 1".into());
        }
        Some(previous) => {
            if event.sequence != previous.sequence + 1 {
                return Err(format!(
                    "trace sequence must continue at {}; got {}",
                    previous.sequence + 1,
                    event.sequence
                ));
            }
            if event.run_id != previous.run_id
                || event.task_id != previous.task_id
                || event.arm != previous.arm
            {
                return Err("trace event identity differs from the previous event".into());
            }
        }
        None => {}
    }

    if let Some(parent) = trace_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create trace directory: {error}"))?;
    }

    let mut line = serde_json::to_string(&event)
        .map_err(|error| format!("cannot serialize event: {error}"))?;
    line.push('\n');
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_path)
        .map_err(|error| format!("cannot open trace {}: {error}", trace_path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("cannot append trace event: {error}"))
}

/// Aggregate a complete trace and current worktree into the frozen G0 result.
///
/// # Errors
///
/// Returns an error when the trace cannot be trusted or repository state cannot
/// be measured.
pub fn aggregate_run(
    repo_root: &Path,
    trace_path: &Path,
    path_policy_path: &Path,
) -> Result<RunResult, String> {
    let repo_root = repo_root
        .canonicalize()
        .map_err(|error| format!("cannot resolve repository root: {error}"))?;
    let events = read_trace(trace_path)?;
    let identity = validate_trace_identity(&events)?;
    let classifier = load_classifier(path_policy_path, &repo_root, identity.arm)?;

    let base_revision = identity.start.base_revision.clone();
    verify_git_revision(&repo_root, &base_revision)?;

    let mut state = AggregationState::new(&repo_root, &base_revision, &classifier);
    for event in &events {
        state.process_event(event)?;
    }

    let final_measurements = state.measure_final_state()?;
    state.finish(identity, final_measurements)
}

fn load_classifier(
    path_policy_path: &Path,
    repo_root: &Path,
    arm: Arm,
) -> Result<PathClassifier, String> {
    let policy_text = fs::read_to_string(path_policy_path).map_err(|error| {
        format!(
            "cannot read path policy {}: {error}",
            path_policy_path.display()
        )
    })?;
    let policy: PathPolicy = serde_json::from_str(&policy_text)
        .map_err(|error| format!("invalid path policy JSON: {error}"))?;
    PathClassifier::from_policy(repo_root, &policy, arm)
}

struct FinalMeasurements {
    accepted_change_tokens: u64,
    changed_bytes: u64,
    changed_lines: u64,
}

struct AggregationState<'a> {
    repo_root: &'a Path,
    base_revision: String,
    classifier: &'a PathClassifier,
    base_cache: BTreeMap<String, Option<String>>,
    last_after: BTreeMap<String, Option<String>>,
    edit_trace: Vec<EditTraceResult>,
    touched_application: BTreeSet<String>,
    cumulative_authored: u64,
    non_application: u64,
    read_operations: u64,
    search_operations: u64,
    edit_operations: u64,
    validation_invocations: u64,
    browser_invocations: u64,
    provider_input_tokens: Option<u64>,
    provider_output_tokens: Option<u64>,
    repairs: Vec<RepairResult>,
    acceptance: BTreeMap<u32, AcceptanceResult>,
    deviations: Vec<ProtocolDeviationResult>,
    core_modification: bool,
    generated_glue_touched: bool,
}

impl<'a> AggregationState<'a> {
    fn new(repo_root: &'a Path, base_revision: String, classifier: &'a PathClassifier) -> Self {
        Self {
            repo_root,
            base_revision,
            classifier,
            base_cache: BTreeMap::new(),
            last_after: BTreeMap::new(),
            edit_trace: Vec::new(),
            touched_application: BTreeSet::new(),
            cumulative_authored: 0,
            non_application: 0,
            read_operations: 0,
            search_operations: 0,
            edit_operations: 0,
            validation_invocations: 0,
            browser_invocations: 0,
            provider_input_tokens: None,
            provider_output_tokens: None,
            repairs: Vec::new(),
            acceptance: BTreeMap::new(),
            deviations: Vec::new(),
            core_modification: false,
            generated_glue_touched: false,
        }
    }

    fn process_event(&mut self, event: &TraceEvent) -> Result<(), String> {
        match &event.payload {
            EventPayload::RepositoryRead { .. } => self.read_operations += 1,
            EventPayload::RepositorySearch { .. } => self.search_operations += 1,
            EventPayload::Validation { .. } => self.validation_invocations += 1,
            EventPayload::Browser { .. } => self.browser_invocations += 1,
            EventPayload::ProviderTokens {
                input_tokens,
                output_tokens,
            } => {
                *self.provider_input_tokens.get_or_insert(0) += *input_tokens;
                *self.provider_output_tokens.get_or_insert(0) += *output_tokens;
            }
            EventPayload::Repair {
                iteration,
                category,
                reason,
                evidence,
                source_paths,
            } => self.repairs.push(RepairResult {
                iteration: *iteration,
                category: *category,
                reason: reason.clone(),
                evidence: evidence.clone(),
                source_paths: source_paths.clone(),
            }),
            EventPayload::Acceptance {
                criterion_index,
                passed,
                evidence,
            } => {
                self.acceptance.insert(
                    *criterion_index,
                    AcceptanceResult {
                        criterion_index: *criterion_index,
                        passed: *passed,
                        evidence: evidence.clone(),
                    },
                );
            }
            EventPayload::ProtocolDeviation {
                code,
                description,
                impact,
            } => self.deviations.push(ProtocolDeviationResult {
                code: code.clone(),
                description: description.clone(),
                impact: *impact,
            }),
            EventPayload::Edit {
                path,
                operation,
                before_text,
                after_text,
            } => self.process_edit(
                event.sequence,
                path,
                *operation,
                before_text.as_deref(),
                after_text.as_deref(),
            )?,
            EventPayload::RunStart { .. } | EventPayload::RunEnd { .. } => {}
        }
        Ok(())
    }

    fn process_edit(
        &mut self,
        sequence: u64,
        path: &str,
        operation: EditOperation,
        before_text: Option<&str>,
        after_text: Option<&str>,
    ) -> Result<(), String> {
        self.edit_operations += 1;
        let path = normalize_relative_path(path)?;
        validate_edit_sides(operation, before_text, after_text)?;

        let expected_before = if let Some(previous) = self.last_after.get(&path) {
            previous.clone()
        } else {
            let base = base_content(self.repo_root, &self.base_revision, &path)?;
            self.base_cache.insert(path.clone(), base.clone());
            base
        };
        if expected_before.as_deref() != before_text {
            return Err(format!(
                "edit sequence {sequence} for `{path}` does not match the previous/base content"
            ));
        }

        let path_class = self.classifier.classify(&path);
        let inserted =
            inserted_token_count(before_text.unwrap_or(""), after_text.unwrap_or("")) as u64;
        self.account_edit(path_class, &path, sequence, inserted);

        self.edit_trace.push(EditTraceResult {
            sequence,
            path: path.clone(),
            path_class,
            operation,
            before_sha256: before_text.map(sha256_hex),
            after_sha256: after_text.map(sha256_hex),
            inserted_tokens: inserted,
        });
        self.last_after.insert(path, after_text.map(str::to_string));
        Ok(())
    }

    fn account_edit(&mut self, path_class: PathClass, path: &str, sequence: u64, inserted: u64) {
        match path_class {
            PathClass::Application => {
                self.cumulative_authored += inserted;
                self.touched_application.insert(path.to_string());
            }
            PathClass::Core => {
                self.non_application += inserted;
                self.core_modification = true;
            }
            PathClass::Generated => {
                self.non_application += inserted;
                self.generated_glue_touched = true;
            }
            PathClass::Harness => {
                self.non_application += inserted;
                self.deviations.push(ProtocolDeviationResult {
                    code: "G0_TRACE_HARNESS_EDIT".into(),
                    description: format!(
                        "harness-owned path `{path}` was edited at sequence {sequence}"
                    ),
                    impact: DeviationImpact::Invalidating,
                });
            }
            PathClass::Other => self.non_application += inserted,
        }
    }

    fn measure_final_state(&mut self) -> Result<FinalMeasurements, String> {
        let final_paths = changed_paths(self.repo_root, &self.base_revision)?;
        let mut measurements = FinalMeasurements {
            accepted_change_tokens: 0,
            changed_bytes: 0,
            changed_lines: 0,
        };

        for path in &final_paths {
            let path_class = self.classifier.classify(path);
            let before = if let Some(cached) = self.base_cache.get(path) {
                cached.clone()
            } else {
                base_content(self.repo_root, &self.base_revision, path)?
            };
            let after = worktree_content(self.repo_root, path)?;
            let before_text = before.as_deref().unwrap_or("");
            let after_text = after.as_deref().unwrap_or("");

            match path_class {
                PathClass::Application => {
                    measurements.accepted_change_tokens +=
                        inserted_token_count(before_text, after_text) as u64;
                    measurements.changed_bytes +=
                        myers_distance(before_text.as_bytes(), after_text.as_bytes()) as u64;
                    let before_lines: Vec<_> = before_text.split_inclusive('\n').collect();
                    let after_lines: Vec<_> = after_text.split_inclusive('\n').collect();
                    measurements.changed_lines +=
                        myers_distance(&before_lines, &after_lines) as u64;
                    self.touched_application.insert(path.clone());

                    if !self.last_after.contains_key(path) {
                        self.deviations.push(ProtocolDeviationResult {
                            code: "G0_TRACE_UNTRACED_APP_CHANGE".into(),
                            description: format!(
                                "application-owned final change `{path}` has no edit event"
                            ),
                            impact: DeviationImpact::Invalidating,
                        });
                    }
                }
                PathClass::Core => self.core_modification = true,
                PathClass::Generated => self.generated_glue_touched = true,
                PathClass::Harness => self.deviations.push(ProtocolDeviationResult {
                    code: "G0_TRACE_HARNESS_FINAL_CHANGE".into(),
                    description: format!(
                        "harness-owned path `{path}` differs from the base revision"
                    ),
                    impact: DeviationImpact::Invalidating,
                }),
                PathClass::Other => {}
            }
        }

        Ok(measurements)
    }

    fn reconcile_last_edits(&mut self) -> Result<(), String> {
        for (path, expected) in &self.last_after {
            let actual = worktree_content(self.repo_root, path)?;
            if &actual != expected {
                self.deviations.push(ProtocolDeviationResult {
                    code: "G0_TRACE_MISSING_EDIT".into(),
                    description: format!(
                        "final worktree content for `{path}` does not match the last edit event"
                    ),
                    impact: DeviationImpact::Invalidating,
                });
            }
        }
        Ok(())
    }

    fn finish(
        mut self,
        identity: TraceIdentity,
        measurements: FinalMeasurements,
    ) -> Result<RunResult, String> {
        self.reconcile_last_edits()?;

        let mut outcome = identity.end.outcome;
        if self
            .deviations
            .iter()
            .any(|deviation| deviation.impact == DeviationImpact::Invalidating)
        {
            outcome = Outcome::Invalidated;
        }

        let wall_clock_ms = identity
            .end
            .ended_unix_ms
            .checked_sub(identity.start.started_unix_ms)
            .ok_or_else(|| "run end time is earlier than run start time".to_string())?;

        Ok(RunResult {
            schema_version: RESULT_SCHEMA_VERSION,
            protocol: PROTOCOL,
            run_id: identity.run_id,
            task_id: identity.task_id,
            arm: identity.arm,
            base_revision: identity.start.base_revision,
            final_revision: identity.end.final_revision,
            started_at: identity.start.started_at,
            ended_at: identity.end.ended_at,
            model: identity.start.model,
            environment: identity.start.environment,
            outcome,
            metrics: Metrics {
                accepted_change_tokens: measurements.accepted_change_tokens,
                cumulative_authored_edit_tokens: self.cumulative_authored,
                non_application_edit_tokens: self.non_application,
                files_touched: self.touched_application.into_iter().collect(),
                changed_bytes: measurements.changed_bytes,
                changed_lines: measurements.changed_lines,
                repository_read_operations: self.read_operations,
                repository_search_operations: self.search_operations,
                edit_operations: self.edit_operations,
                validation_invocations: self.validation_invocations,
                browser_invocations: self.browser_invocations,
                acceptance_loop_actions: self.validation_invocations + self.browser_invocations,
                provider_input_tokens: self.provider_input_tokens,
                provider_output_tokens: self.provider_output_tokens,
                wall_clock_ms,
                core_modification: self.core_modification,
                generated_glue_touched: self.generated_glue_touched,
            },
            repairs: self.repairs,
            acceptance: self.acceptance.into_values().collect(),
            edit_trace: self.edit_trace,
            protocol_deviations: self.deviations,
        })
    }
}

struct TraceIdentity {
    run_id: String,
    task_id: String,
    arm: Arm,
    start: RunStartData,
    end: RunEndData,
}

struct RunStartData {
    base_revision: String,
    started_at: String,
    started_unix_ms: u64,
    model: ModelMetadata,
    environment: EnvironmentMetadata,
}

struct RunEndData {
    ended_at: String,
    ended_unix_ms: u64,
    outcome: Outcome,
    final_revision: Option<String>,
}

fn read_trace(path: &Path) -> Result<Vec<TraceEvent>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read trace {}: {error}", path.display()))?;
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: TraceEvent = serde_json::from_str(line)
            .map_err(|error| format!("invalid trace JSON on line {}: {error}", index + 1))?;
        validate_event_shape(&event)?;
        events.push(event);
    }
    if events.is_empty() {
        return Err("trace is empty".into());
    }
    Ok(events)
}

fn validate_trace_identity(events: &[TraceEvent]) -> Result<TraceIdentity, String> {
    for (index, event) in events.iter().enumerate() {
        let expected = index as u64 + 1;
        if event.sequence != expected {
            return Err(format!(
                "trace sequence must be contiguous from 1; expected {expected}, got {}",
                event.sequence
            ));
        }
    }

    let first = &events[0];
    let last = events
        .last()
        .expect("non-empty trace is validated before identity");

    for event in events {
        if event.run_id != first.run_id || event.task_id != first.task_id || event.arm != first.arm
        {
            return Err(format!(
                "trace event {} has inconsistent run/task/arm identity",
                event.sequence
            ));
        }
    }

    let EventPayload::RunStart {
        base_revision,
        started_at,
        started_unix_ms,
        model,
        environment,
    } = &first.payload
    else {
        return Err("first trace event must be run_start".into());
    };
    let EventPayload::RunEnd {
        ended_at,
        ended_unix_ms,
        outcome,
        final_revision,
    } = &last.payload
    else {
        return Err("last trace event must be run_end".into());
    };

    if events[1..events.len().saturating_sub(1)]
        .iter()
        .any(|event| {
            matches!(
                &event.payload,
                EventPayload::RunStart { .. } | EventPayload::RunEnd { .. }
            )
        })
    {
        return Err("run_start/run_end may only appear as first/last events".into());
    }

    Ok(TraceIdentity {
        run_id: first.run_id.clone(),
        task_id: first.task_id.clone(),
        arm: first.arm,
        start: RunStartData {
            base_revision: base_revision.clone(),
            started_at: started_at.clone(),
            started_unix_ms: *started_unix_ms,
            model: model.as_ref().clone(),
            environment: environment.as_ref().clone(),
        },
        end: RunEndData {
            ended_at: ended_at.clone(),
            ended_unix_ms: *ended_unix_ms,
            outcome: *outcome,
            final_revision: final_revision.clone(),
        },
    })
}

fn validate_event_shape(event: &TraceEvent) -> Result<(), String> {
    if event.schema_version != TRACE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported trace schema version {}; expected {}",
            event.schema_version, TRACE_SCHEMA_VERSION
        ));
    }
    if event.run_id.is_empty() || event.task_id.is_empty() {
        return Err("trace runId and taskId must be non-empty".into());
    }
    if !matches!(
        event.task_id.as_str(),
        "G0-D01" | "G0-D02" | "G0-D03" | "G0-D04" | "G0-D05" | "G0-D06"
    ) {
        return Err(format!("unknown frozen G0 task `{}`", event.task_id));
    }
    Ok(())
}

fn validate_edit_sides(
    operation: EditOperation,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<(), String> {
    let valid = match operation {
        EditOperation::Create => before.is_none() && after.is_some(),
        EditOperation::Update => before.is_some() && after.is_some(),
        EditOperation::Delete => before.is_some() && after.is_none(),
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "edit operation {operation:?} has inconsistent beforeText/afterText sides"
        ))
    }
}

fn normalize_relative_path(path: &str) -> Result<String, String> {
    let path = path.replace('\\', "/");
    let parsed = Path::new(&path);
    if parsed.is_absolute()
        || parsed.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("trace path escapes repository root: {path}"));
    }
    if path.is_empty() {
        return Err("trace path cannot be empty".into());
    }
    Ok(path.trim_start_matches("./").to_string())
}

fn verify_git_revision(repo_root: &Path, revision: &str) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--verify", &format!("{revision}^{{commit}}")])
        .output()
        .map_err(|error| format!("cannot execute git: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "base revision `{revision}` is not a valid commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn base_content(repo_root: &Path, revision: &str, path: &str) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["show", &format!("{revision}:{path}")])
        .output()
        .map_err(|error| format!("cannot execute git show: {error}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(Some)
            .map_err(|_| format!("application/trace file `{path}` is not UTF-8"))
    } else {
        Ok(None)
    }
}

fn worktree_content(repo_root: &Path, path: &str) -> Result<Option<String>, String> {
    let full = repo_root.join(path);
    if !full.exists() {
        return Ok(None);
    }
    fs::read_to_string(&full)
        .map(Some)
        .map_err(|error| format!("cannot read worktree file `{path}`: {error}"))
}

fn changed_paths(repo_root: &Path, revision: &str) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();

    let diff = Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--no-renames", "--name-only", "-z", revision, "--"])
        .output()
        .map_err(|error| format!("cannot execute git diff: {error}"))?;
    if !diff.status.success() {
        return Err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&diff.stderr).trim()
        ));
    }
    parse_nul_paths(&diff.stdout, &mut paths)?;

    let untracked = Command::new("git")
        .current_dir(repo_root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .map_err(|error| format!("cannot execute git ls-files: {error}"))?;
    if !untracked.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&untracked.stderr).trim()
        ));
    }
    parse_nul_paths(&untracked.stdout, &mut paths)?;

    Ok(paths)
}

fn parse_nul_paths(bytes: &[u8], paths: &mut BTreeSet<String>) -> Result<(), String> {
    for raw in bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = std::str::from_utf8(raw)
            .map_err(|_| "git returned a non-UTF-8 path, unsupported by G0".to_string())?;
        paths.insert(normalize_relative_path(path)?);
    }
    Ok(())
}

#[must_use]
pub fn inserted_token_count(before: &str, after: &str) -> usize {
    let before_tokens: Vec<_> = TOKEN_RE
        .find_iter(before)
        .map(|item| item.as_str())
        .collect();
    let after_tokens: Vec<_> = TOKEN_RE
        .find_iter(after)
        .map(|item| item.as_str())
        .collect();
    let distance = myers_distance(&before_tokens, &after_tokens);

    if after_tokens.len() >= before_tokens.len() {
        (distance + after_tokens.len() - before_tokens.len()) / 2
    } else {
        distance
            .checked_sub(before_tokens.len() - after_tokens.len())
            .unwrap_or(0)
            / 2
    }
}

fn myers_distance<T: Eq>(before: &[T], after: &[T]) -> usize {
    let Ok(before_len) = isize::try_from(before.len()) else {
        return before.len().saturating_add(after.len());
    };
    let Ok(after_len) = isize::try_from(after.len()) else {
        return before.len().saturating_add(after.len());
    };
    let Some(max_distance) = before_len.checked_add(after_len) else {
        return before.len().saturating_add(after.len());
    };
    if max_distance == 0 {
        return 0;
    }

    let offset = max_distance + 1;
    let Some(vector_len_signed) = max_distance
        .checked_mul(2)
        .and_then(|value| value.checked_add(3))
    else {
        return before.len().saturating_add(after.len());
    };
    let Ok(vector_len) = usize::try_from(vector_len_signed) else {
        return before.len().saturating_add(after.len());
    };
    let mut frontier = vec![0_isize; vector_len];
    let offset_plus_one = usize::try_from(offset + 1).expect("positive Myers offset");
    frontier[offset_plus_one] = 0;

    for edit_depth in 0..=max_distance {
        let mut diagonal = -edit_depth;
        while diagonal <= edit_depth {
            let index =
                usize::try_from(diagonal + offset).expect("non-negative Myers frontier index");
            let mut before_position = if diagonal == -edit_depth
                || (diagonal != edit_depth && frontier[index - 1] < frontier[index + 1])
            {
                frontier[index + 1]
            } else {
                frontier[index - 1] + 1
            };
            let mut after_position = before_position - diagonal;

            while before_position < before_len
                && after_position < after_len
                && before[usize::try_from(before_position).expect("before position")]
                    == after[usize::try_from(after_position).expect("after position")]
            {
                before_position += 1;
                after_position += 1;
            }
            frontier[index] = before_position;

            if before_position >= before_len && after_position >= after_len {
                return usize::try_from(edit_depth).expect("non-negative edit distance");
            }
            diagonal += 2;
        }
    }

    before.len().saturating_add(after.len())
}

fn sha256_hex(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(output, "{byte:02x}").expect("writing hash to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Arm, PathClass, PathClassifier, PathPolicy, inserted_token_count, myers_distance};

    #[test]
    fn frozen_tokenizer_counts_unicode_words_and_punctuation() {
        assert_eq!(inserted_token_count("", "café = customer.name;"), 6);
    }

    #[test]
    fn pure_deletion_adds_zero_authored_tokens() {
        assert_eq!(inserted_token_count("alpha beta", "alpha"), 0);
    }

    #[test]
    fn reverted_edit_counts_cumulatively_but_not_in_final_change() {
        let first = inserted_token_count("alpha", "alpha beta");
        let revert = inserted_token_count("alpha beta", "alpha");
        let final_change = inserted_token_count("alpha", "alpha");

        assert_eq!(first + revert, 1);
        assert_eq!(final_change, 0);
    }

    #[test]
    fn myers_distance_counts_insert_and_delete_for_replacement() {
        assert_eq!(myers_distance(&["a"], &["b"]), 2);
        assert_eq!(myers_distance(&["a", "b"], &["a", "b"]), 0);
    }

    #[test]
    fn frozen_path_policy_classifies_primary_arms() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let policy_text = std::fs::read_to_string(root.join("experiments/g0/path-policy.json"))
            .expect("frozen path policy");
        let policy: PathPolicy = serde_json::from_str(&policy_text).expect("path policy JSON");

        let b_full = PathClassifier::from_policy(&root, &policy, Arm::BFull).expect("B-full");
        assert_eq!(
            b_full.classify("baselines/b-full/src/pages/customers.tsx"),
            PathClass::Application
        );
        assert_eq!(
            b_full.classify("baselines/b-full/src/api/generated/schema.d.ts"),
            PathClass::Generated
        );
        assert_eq!(
            b_full.classify("baselines/b-full/package.json"),
            PathClass::Harness
        );

        let uiko = PathClassifier::from_policy(&root, &policy, Arm::CUiko).expect("uiko");
        assert_eq!(
            uiko.classify("fixtures/support-console/features/customers/detail.jsonc"),
            PathClass::Application
        );
        assert_eq!(
            uiko.classify("fixtures/support-console/integrations/crm.jsonc"),
            PathClass::Harness
        );
        assert_eq!(
            uiko.classify("crates/uiko-core/src/lib.rs"),
            PathClass::Core
        );
    }
}
