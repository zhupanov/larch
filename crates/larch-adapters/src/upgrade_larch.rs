use crate::{
    TokioProcessRunner,
    file_io::atomic_write_bytes,
    filesystem::{PathIntent, PluginRoot},
    runtime::{Cancellation, LarchRuntime},
};
use larch_core::{
    ActiveRootState, ChildEnvironment, ExternalProcessRunner, ExternalProgram,
    InstalledVersionState, LarchProgram, MarketplaceState, ProcessOutput, ProcessRequest,
    UpgradeDisposition, VendorProgram, classify_upgrade, env,
};
use serde::Deserialize;
use std::{
    env as process_env,
    ffi::OsString,
    fs,
    num::NonZeroUsize,
    path::{Component, Path, PathBuf},
    time::Duration,
};

/// The marketplace descriptor, read from `main` on purpose.
///
/// The descriptor is a pointer, not installed content: it names the branch that
/// installed plugin content is pinned to. Reading it from `main` keeps that
/// pointer editable without a release, while the content it points at stays
/// pinned to one commit per version.
pub const MARKETPLACE_SOURCE: &str =
    "https://raw.githubusercontent.com/character-ai/larch/main/.claude-plugin/marketplace.json";
const CACHE_RELATIVE: &str = ".claude/plugins/cache/larch-local/larch";
const MARKETPLACE_RELATIVE: &str = ".claude/plugins/marketplaces/larch-local";
/// Claude Code's install registry: the harness-level pointer that names the
/// active plugin root for every new session. `claude plugin install|update`
/// rewrites it, and no Claude command reverts it to a previous version, so the
/// rollback in `recover_active_root` restores a byte-identical pre-flip snapshot.
const INSTALLED_REGISTRY_RELATIVE: &str = ".claude/plugins/installed_plugins.json";
const INSTALLED_REGISTRY_NAME: &str = "installed_plugins.json";
const LARCH_ID: &str = "larch@larch-local";
/// The bootstrap's proof that the branch `.claude-plugin/marketplace.json` pins
/// installed plugin content to is at the same commit as the release being
/// installed. Written by `verify_release_pin` in `scripts/larch.sh`.
const PREFLIGHT_PIN_MARKER: &str = "LARCH_PREFLIGHT_PIN_VERIFIED=true";

#[derive(Debug)]
pub struct Failure {
    pub code: u8,
    pub message: String,
}

impl Failure {
    fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct InstalledEntry {
    id: Option<String>,
    version: Option<String>,
    #[serde(rename = "installPath")]
    install_path: Option<PathBuf>,
}

#[derive(Deserialize)]
struct MarketplaceEntry {
    name: Option<String>,
    source: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    version: Option<String>,
}

#[derive(Deserialize)]
struct Identity {
    schema_version: Option<u64>,
    version: Option<String>,
}

struct Context {
    runtime: LarchRuntime,
    runner: TokioProcessRunner,
    cancellation: Cancellation,
    cwd: PathBuf,
    home: PathBuf,
    plugin_data: Option<PathBuf>,
}

impl Context {
    fn load() -> Result<Self, Failure> {
        let cwd = process_env::current_dir().map_err(|error| {
            Failure::new(1, format!("Unable to resolve current directory: {error}"))
        })?;
        let home = absolute_env(env::HOME)?;
        let plugin_data = process_env::var_os(env::CLAUDE_PLUGIN_DATA)
            .map(PathBuf::from)
            .map(|path| validate_absolute(env::CLAUDE_PLUGIN_DATA, path))
            .transpose()?;
        let runtime = LarchRuntime::new()
            .map_err(|error| Failure::new(1, format!("Unable to initialize runtime: {error}")))?;
        Ok(Self {
            runtime,
            runner: TokioProcessRunner::default(),
            cancellation: Cancellation::new(),
            cwd,
            home,
            plugin_data,
        })
    }

    fn process(
        &self,
        program: ExternalProgram,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
        timeout: Duration,
        plugin_root: Option<&Path>,
        bootstrap_environment: bool,
    ) -> Result<ProcessOutput, Failure> {
        let mut request = ProcessRequest::new(
            program,
            arguments,
            self.cwd.clone(),
            timeout,
            Duration::from_secs(5),
            NonZeroUsize::new(4 * 1024 * 1024).expect("fixed output limit is non-zero"),
        )
        .map_err(|error| Failure::new(1, error.to_string()))?;
        if let Some(plugin_data) = &self.plugin_data {
            request = request
                .with_environment(ChildEnvironment::ClaudePluginData, plugin_data.as_os_str());
        }
        if let Some(root) = plugin_root {
            request =
                request.with_environment(ChildEnvironment::ClaudePluginRoot, root.as_os_str());
        }
        if bootstrap_environment {
            for (key, name) in [
                (ChildEnvironment::GhConfigDir, env::GH_CONFIG_DIR),
                (ChildEnvironment::XdgConfigHome, env::XDG_CONFIG_HOME),
            ] {
                if let Some(value) = process_env::var_os(name) {
                    request = request.with_environment(key, value);
                }
            }
        }
        self.runtime
            .block_on(self.runner.run(request, &self.cancellation))
            .map_err(|error| Failure::new(1, error.to_string()))
    }

    fn claude(&self, arguments: &[&str]) -> Result<ProcessOutput, Failure> {
        self.process(
            ExternalProgram::Vendor(VendorProgram::Claude),
            arguments,
            Duration::from_secs(120),
            None,
            false,
        )
    }

    fn installed_entries(&self) -> Vec<InstalledEntry> {
        let Ok(output) = self.claude(&["plugin", "list", "--json"]) else {
            return Vec::new();
        };
        if !output.status().success() || output.stdout_truncated() {
            return Vec::new();
        }
        serde_json::from_slice::<Vec<InstalledEntry>>(output.stdout())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.id.as_deref() == Some(LARCH_ID))
            .collect()
    }

    fn installed_version(&self) -> Option<String> {
        let mut versions = self
            .installed_entries()
            .into_iter()
            .filter_map(|entry| entry.version)
            .filter(|version| safe_version(version))
            .collect::<Vec<_>>();
        versions.sort();
        versions.dedup();
        (versions.len() == 1).then(|| versions.remove(0))
    }

    fn cache_parent(&self) -> PathBuf {
        self.home.join(CACHE_RELATIVE)
    }

    fn resolve_installed_root(&self, version: &str) -> Option<PathBuf> {
        if !safe_version(version) {
            return None;
        }
        let mut roots = self
            .installed_entries()
            .into_iter()
            .filter(|entry| entry.version.as_deref() == Some(version))
            .filter_map(|entry| entry.install_path)
            .collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        if roots.len() != 1 {
            return None;
        }
        let root = roots.remove(0);
        let resolved = real_directory(&root)?;
        let parent = fs::canonicalize(self.cache_parent()).ok()?;
        if resolved.parent()? != parent || resolved.file_name()?.to_str()? != version {
            return None;
        }
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(resolved.join(".claude-plugin/plugin.json")).ok()?)
                .ok()?;
        (manifest.version.as_deref() == Some(version)).then_some(resolved)
    }

    fn marketplace_matches(&self) -> bool {
        let Ok(output) = self.claude(&["plugin", "marketplace", "list", "--json"]) else {
            return false;
        };
        if !output.status().success() || output.stdout_truncated() {
            return false;
        }
        let Ok(entries) = serde_json::from_slice::<Vec<MarketplaceEntry>>(output.stdout()) else {
            return false;
        };
        let matches = entries
            .iter()
            .filter(|entry| entry.name.as_deref() == Some("larch-local"))
            .collect::<Vec<_>>();
        matches.len() == 1
            && matches[0].source.as_deref() == Some("url")
            && matches[0].url.as_deref() == Some(MARKETPLACE_SOURCE)
    }

    fn bootstrap(
        &self,
        root: &Path,
        arguments: &[&str],
        timeout: Duration,
    ) -> Result<ProcessOutput, Failure> {
        if self.plugin_data.is_none() {
            return Err(Failure::new(
                1,
                "CLAUDE_PLUGIN_DATA is required. Set it to an absolute, symlink-free path \
                 for bounded bootstrap staging; see the documented local-dev pattern in \
                 docs/installation-and-setup.md. On macOS, /tmp and /var are symlinks.",
            ));
        }
        let script = safe_root_file(root, "scripts/larch.sh")?;
        let root = script
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Failure::new(1, "Bootstrap path has no plugin root"))?;
        let program =
            LarchProgram::bootstrap(root).map_err(|error| Failure::new(1, error.to_string()))?;
        self.process(
            ExternalProgram::Larch(program),
            arguments,
            timeout,
            Some(root),
            true,
        )
    }

    fn bootstrap_installed_root(&self, root: &Path, version: &str) -> bool {
        let Ok(output) =
            self.bootstrap(root, &["bootstrap", "self-check"], Duration::from_secs(600))
        else {
            return false;
        };
        relay(&output);
        let Ok(binary) = safe_root_file(root, "bin/larch") else {
            return false;
        };
        if !output.status().success() || output.stdout_truncated() || !executable_file(&binary) {
            return false;
        }
        let Ok(program) = LarchProgram::binary(root) else {
            return false;
        };
        let Ok(direct) = self.process(
            ExternalProgram::Larch(program),
            ["bootstrap", "self-check"],
            Duration::from_secs(30),
            Some(root),
            false,
        ) else {
            return false;
        };
        if !direct.status().success() || direct.stdout() != output.stdout() {
            return false;
        }
        serde_json::from_slice::<Identity>(direct.stdout()).is_ok_and(|identity| {
            identity.schema_version == Some(1) && identity.version.as_deref() == Some(version)
        })
    }

    fn installed_registry(&self) -> PathBuf {
        self.home.join(INSTALLED_REGISTRY_RELATIVE)
    }

    /// Capture the install registry before `claude plugin install|update`
    /// flips the active root. `None` means there is nothing to restore: the
    /// registry is absent, or it is not a regular file this driver may rewrite.
    fn installed_registry_snapshot(&self) -> Option<RegistrySnapshot> {
        let path = self.installed_registry();
        let metadata = fs::symlink_metadata(&path).ok()?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return None;
        }
        let bytes = fs::read(&path).ok()?;
        Some(RegistrySnapshot {
            bytes,
            mode: unix_mode(&metadata),
        })
    }

    /// Put the pre-flip registry back through the confined atomic writer.
    ///
    /// `post_flip` is the registry as `claude plugin install|update` left it.
    /// A registry that differs from it now was rewritten by another process,
    /// and restoring the snapshot would discard that write, so refuse.
    fn restore_installed_registry(
        &self,
        snapshot: &RegistrySnapshot,
        post_flip: Option<&RegistrySnapshot>,
    ) -> Result<(), String> {
        let path = self.installed_registry();
        let live = self.installed_registry_snapshot();
        if live.as_ref().map(|state| &state.bytes) != post_flip.map(|state| &state.bytes) {
            return Err(format!(
                "{} changed after the plugin install; another process owns the newer content",
                path.display()
            ));
        }
        let plugins = path
            .parent()
            .ok_or_else(|| "install registry has no parent directory".to_owned())?;
        let root = PluginRoot::resolve(Some(plugins)).map_err(|error| error.to_string())?;
        let confined = root
            .confine(INSTALLED_REGISTRY_NAME, PathIntent::Write)
            .map_err(|error| error.to_string())?;
        atomic_write_bytes(&confined, &snapshot.bytes, snapshot.mode)
            .map_err(|error| error.to_string())
    }
}

struct RegistrySnapshot {
    bytes: Vec<u8>,
    mode: u32,
}

/// What the driver did about the active root after a post-flip verification failure.
#[derive(Debug)]
enum ActiveRootRecovery {
    /// The registry names `prior` again and that root re-verified.
    RolledBack { prior: String },
    /// The install did not move the pointer to another version, so there is
    /// no earlier registry state worth restoring.
    PointerUnchanged,
    /// No pre-flip snapshot exists, so the pointer stays on the new root.
    NoSnapshot,
    /// The restore itself failed and the pointer stays on the new root.
    RestoreFailed(String),
    /// The registry was restored, but `claude plugin list --json` or the prior
    /// root's binary did not confirm the prior version.
    RestoreUnverified,
}

pub fn sparse_dirs() {
    println!(".claude-plugin");
}

/// Resolve and print the cache root used by release Step 7.
///
/// # Errors
/// Returns a failure when the environment or installed metadata does not name one safe root.
pub fn release_step7_root(current_version: Option<&str>) -> Result<(), Failure> {
    let context = Context::load()?;
    let active = process_env::var_os(env::CLAUDE_PLUGIN_ROOT).map(PathBuf::from);
    if let Some(root) = active.filter(|root| cache_shaped(root, &context.cache_parent())) {
        println!("RESOLVED_ROOT={}", root.display());
        return Ok(());
    }
    let parent = context.cache_parent();
    if let Some(version) = context.installed_version() {
        let root = parent.join(version);
        if real_directory(&root).is_some() {
            println!("RESOLVED_ROOT={}", root.display());
            return Ok(());
        }
    }
    let sole = sole_version_directory(&parent);
    if current_version
        .filter(|value| safe_version(value))
        .is_some_and(|version| sole.as_deref() == Some(parent.join(version).as_path()))
        && let Some(root) = sole
    {
        println!("RESOLVED_ROOT={}", root.display());
        return Ok(());
    }
    Err(Failure::new(1, "ERROR=Unable to resolve larch cache root"))
}

/// Upgrade the installed plugin and executable to the verified stable release.
///
/// # Errors
/// Returns a classified command failure when a required precondition, child, or postcondition fails.
pub fn run(plugin_root: Option<&Path>) -> Result<(), Failure> {
    let context = Context::load()?;
    let plugin_root = plugin_root.map_or_else(
        || {
            process_env::var_os(env::CLAUDE_PLUGIN_ROOT)
                .map_or_else(|| context.cwd.clone(), PathBuf::from)
        },
        Path::to_path_buf,
    );
    let plugin_root = real_directory(&plugin_root)
        .ok_or_else(|| Failure::new(1, "Upgrade cannot resolve a safe CLAUDE_PLUGIN_ROOT."))?;
    // The release preflight runs `<root>/scripts/larch.sh --preflight-release`,
    // a flag only the version being installed supports. Use the driver's own
    // root (this running larch), not the possibly-older install target
    // `plugin_root`. Upgrading from a pre-`--preflight-release` version (e.g.
    // 53.x) otherwise fails with "unexpected argument".
    let driver_root = process_env::var_os(env::CLAUDE_PLUGIN_ROOT)
        .map(PathBuf::from)
        .and_then(|root| real_directory(&root))
        .unwrap_or_else(|| plugin_root.clone());
    let installed_version = plugin_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_owned();
    let current = context
        .installed_version()
        .unwrap_or_else(|| installed_version.clone());
    let marketplace_will_reconcile = !context.marketplace_matches();
    let latest = expected_or_latest(&context, &plugin_root)?;

    let disposition = upgrade_disposition(
        &current,
        &latest,
        marketplace_will_reconcile,
        cache_shaped(&plugin_root, &context.cache_parent()),
        &installed_version,
    );
    if disposition == UpgradeDisposition::NoOpRepair {
        return repair_no_op(&context, &current);
    }
    if disposition == UpgradeDisposition::ActiveOldSession {
        eprintln!();
        eprintln!(
            "Installed metadata is already at latest stable larch release ({current}), but this Claude Code session is still running cached larch {installed_version}. Refreshing the install and requiring restart..."
        );
    } else if disposition == UpgradeDisposition::MarketplaceMigration {
        eprintln!();
        eprintln!(
            "Already at latest stable larch release ({current}), but the marketplace still uses the legacy source. Migrating it to the runtime-only source and reinstalling..."
        );
    } else {
        eprintln!("Upgrading larch from {installed_version} to {latest}...");
    }
    preflight(&context, &driver_root, &latest)?;
    let mode = refresh_marketplace(&context, marketplace_will_reconcile)?;
    let actual = install_and_verify(&context, mode, &current, &latest)?;
    if marketplace_will_reconcile {
        eprintln!(
            "LARCH_MARKETPLACE_RECONCILED={}",
            context.marketplace_matches()
        );
    }
    eprintln!("LARCH_RESTART_REQUIRED=true");
    if actual != current {
        eprintln!("LARCH_NEW_VERSION_INSTALLED=true");
    }
    eprintln!();
    eprintln!("Installed larch plugin version:");
    if let Ok(listed) = context.claude(&["plugin", "list"]) {
        relay(&listed);
    }
    eprintln!();
    eprintln!("Upgrade complete. Restart Claude Code to apply the new version.");
    Ok(())
}

/// Run `claude plugin install|update`, then prove the new root's executable.
///
/// Returns the installed version. A post-flip verification failure routes
/// through `recover_active_root` so new sessions never stay on a root whose
/// `bin/larch` never materialized.
fn install_and_verify(
    context: &Context,
    mode: RefreshMode,
    current: &str,
    latest: &str,
) -> Result<String, Failure> {
    eprintln!("Installing the preflighted larch plugin release...");
    let verb = if mode == RefreshMode::Install {
        "install"
    } else {
        "update"
    };
    // `claude plugin install|update` flips the active root for every new
    // session before this driver can materialize the new root's `bin/larch`.
    // Capture the registry first so a post-flip failure can put the pointer
    // back instead of stranding new sessions on a root with no executable.
    let registry_snapshot = context.installed_registry_snapshot();
    let install = match context.claude(&["plugin", verb, LARCH_ID]) {
        Ok(output) => output,
        Err(error) => {
            recovery();
            return Err(error);
        }
    };
    relay(&install);
    if !install.status().success() {
        eprintln!("Plugin install failed. The prior cache root was not modified.");
        recovery();
        return Err(Failure::new(exit_code(&install), "Plugin install failed."));
    }
    let post_flip_registry = context.installed_registry_snapshot();
    let actual = context.installed_version().unwrap_or_default();
    let root = (actual == latest)
        .then(|| context.resolve_installed_root(&actual))
        .flatten();
    if root
        .as_deref()
        .is_none_or(|root| !context.bootstrap_installed_root(root, &actual))
    {
        eprintln!(
            "Upgrade incomplete: expected plugin and binary version {latest} in the newly installed cache root."
        );
        let outcome = recover_active_root(
            context,
            registry_snapshot.as_ref(),
            post_flip_registry.as_ref(),
            current,
            &actual,
        );
        let unverified_root = root.unwrap_or_else(|| context.cache_parent().join(latest));
        return Err(report_active_root_recovery(&outcome, &unverified_root));
    }
    Ok(actual)
}

fn upgrade_disposition(
    current: &str,
    latest: &str,
    marketplace_will_reconcile: bool,
    active_root_is_cache_shaped: bool,
    active_root_version: &str,
) -> UpgradeDisposition {
    let version = if current == latest {
        InstalledVersionState::Current
    } else {
        InstalledVersionState::Different
    };
    let marketplace = if marketplace_will_reconcile {
        MarketplaceState::Legacy
    } else {
        MarketplaceState::RuntimeOnly
    };
    let active_root = if !active_root_is_cache_shaped {
        ActiveRootState::NonCache
    } else if active_root_version == latest {
        ActiveRootState::CurrentCache
    } else {
        ActiveRootState::OldCache
    };
    classify_upgrade(version, marketplace, active_root)
}

fn repair_no_op(context: &Context, current: &str) -> Result<(), Failure> {
    let root = context.resolve_installed_root(current);
    if root
        .as_deref()
        .is_none_or(|root| !context.bootstrap_installed_root(root, current))
    {
        recovery();
        return Err(Failure::new(1, "Binary verification failed."));
    }
    eprintln!();
    eprintln!(
        "Already at latest stable larch release ({current}). Binary verification passed. No upgrade needed."
    );
    Ok(())
}

fn expected_or_latest(context: &Context, root: &Path) -> Result<String, Failure> {
    if let Ok(version) = process_env::var(env::LARCH_EXPECTED_STABLE_VERSION)
        && safe_version(&version)
    {
        return Ok(version);
    }
    let output =
        match context.bootstrap(root, &["--latest-stable-version"], Duration::from_secs(120)) {
            Ok(output) => output,
            Err(error) => {
                recovery();
                return Err(error);
            }
        };
    relay(&output);
    if output.status().success() && !output.stdout_truncated() {
        for line in String::from_utf8_lossy(output.stdout()).lines() {
            if let Some(version) = line.strip_prefix("LARCH_STABLE_VERSION=")
                && safe_version(version)
            {
                return Ok(version.to_owned());
            }
        }
    }
    eprintln!(
        "Latest stable release could not be determined. Upgrade stopped before changing plugin state."
    );
    recovery();
    Err(Failure::new(1, "Stable release resolution failed."))
}

/// Verify the release before any plugin state changes.
///
/// The preflight must prove two things: the immutable release for `version`
/// verifies end to end, and the marketplace-pinned branch is at that release's
/// tagged commit. Requiring the pin proof here, rather than after
/// `claude plugin install`, keeps a content-and-binary mismatch from ever
/// becoming the active installation.
fn preflight(context: &Context, root: &Path, version: &str) -> Result<(), Failure> {
    eprintln!("Preflighting immutable larch release v{version}...");
    let output = match context.bootstrap(
        root,
        &["--preflight-release", version],
        Duration::from_secs(600),
    ) {
        Ok(output) => output,
        Err(error) => {
            recovery();
            return Err(error);
        }
    };
    relay(&output);
    let version_marker = format!("LARCH_PREFLIGHT_VERSION={version}");
    if output.status().success() && !output.stdout_truncated() {
        let stdout = String::from_utf8_lossy(output.stdout());
        let reported = |marker: &str| stdout.lines().any(|line| line == marker);
        match (reported(&version_marker), reported(PREFLIGHT_PIN_MARKER)) {
            (true, true) => return Ok(()),
            (true, false) => {
                eprintln!(
                    "Upgrade stopped because the release preflight did not prove that the installed plugin content and the release executable come from one commit."
                );
                recovery();
                return Err(Failure::new(1, "Release pin verification is missing."));
            }
            _ => {}
        }
    }
    eprintln!("Upgrade stopped because stable release preflight failed.");
    recovery();
    Err(Failure::new(1, "Stable release preflight failed."))
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RefreshMode {
    Install,
    Update,
}

fn refresh_marketplace(context: &Context, reconcile: bool) -> Result<RefreshMode, Failure> {
    if !reconcile {
        eprintln!("Refreshing the runtime-only larch marketplace...");
        let output = match context.claude(&["plugin", "marketplace", "update", "larch-local"]) {
            Ok(output) => output,
            Err(error) => {
                recovery();
                return Err(error);
            }
        };
        relay(&output);
        if output.status().success() {
            return Ok(RefreshMode::Update);
        }
        eprintln!("Marketplace refresh failed. The prior plugin cache root was not changed.");
        recovery();
        return Err(Failure::new(1, "Marketplace refresh failed."));
    }
    eprintln!("Migrating the larch marketplace to the runtime-only remote source...");
    let clone = context.home.join(MARKETPLACE_RELATIVE);
    if fs::symlink_metadata(&clone).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        recovery();
        return Err(Failure::new(
            1,
            "Marketplace migration refused a symlinked marketplace clone.",
        ));
    }
    let removed = match context.claude(&["plugin", "marketplace", "remove", "larch-local"]) {
        Ok(output) => output,
        Err(error) => {
            recovery();
            return Err(error);
        }
    };
    relay(&removed);
    if !removed.status().success() {
        recovery();
        return Err(Failure::new(
            1,
            "Marketplace reconciliation stopped because the legacy registration could not be removed.",
        ));
    }
    if clone.exists() {
        fs::remove_dir_all(&clone).map_err(|error| {
            recovery();
            Failure::new(
                1,
                format!(
                    "Warning: failed to remove marketplace clone '{}': {error}",
                    clone.display()
                ),
            )
        })?;
    }
    let added = match context.claude(&["plugin", "marketplace", "add", MARKETPLACE_SOURCE]) {
        Ok(output) => output,
        Err(error) => {
            recovery();
            return Err(error);
        }
    };
    relay(&added);
    if added.status().success() && context.marketplace_matches() {
        Ok(RefreshMode::Install)
    } else {
        recovery();
        Err(Failure::new(1, "Marketplace migration failed."))
    }
}

fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn absolute_env(name: &'static str) -> Result<PathBuf, Failure> {
    let path = process_env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| Failure::new(1, format!("{name} is required")))?;
    validate_absolute(name, path)
}

fn validate_absolute(name: &'static str, path: PathBuf) -> Result<PathBuf, Failure> {
    if path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        Ok(path)
    } else {
        Err(Failure::new(
            1,
            format!("{name} must be an absolute safe path"),
        ))
    }
}

fn real_directory(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    (!metadata.file_type().is_symlink() && metadata.is_dir())
        .then(|| fs::canonicalize(path).ok())
        .flatten()
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn safe_root_file(root: &Path, relative: &str) -> Result<PathBuf, Failure> {
    let root = real_directory(root).ok_or_else(|| Failure::new(1, "Plugin root is unsafe"))?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path).map_err(|_| {
        Failure::new(
            1,
            format!("Required larch executable is missing: {}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Failure::new(
            1,
            format!("Required larch executable is unsafe: {}", path.display()),
        ));
    }
    Ok(path)
}

fn cache_shaped(root: &Path, parent: &Path) -> bool {
    let Some(root) = real_directory(root) else {
        return false;
    };
    let parent = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    root.parent() == Some(parent.as_path())
        && root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(safe_version)
}

fn sole_version_directory(parent: &Path) -> Option<PathBuf> {
    let mut roots = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(safe_version)
                && real_directory(path).is_some()
        })
        .collect::<Vec<_>>();
    (roots.len() == 1).then(|| roots.remove(0))
}

fn relay(output: &ProcessOutput) {
    eprint!("{}", output.safe_stderr().as_str());
    print!("{}", output.safe_stdout().as_str());
}

fn exit_code(output: &ProcessOutput) -> u8 {
    output
        .status()
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1)
}

/// Recovery text for failures before `claude plugin install|update` runs.
///
/// Every caller stops before the active root moves, so the running session
/// and the prior cache root are still what new sessions resolve. Post-flip
/// failures report through `report_active_root_recovery` instead.
fn recovery() {
    eprintln!();
    eprintln!(
        "Recovery: retry /upgrade-larch. The running session and prior cache root remain usable."
    );
    eprintln!("If marketplace metadata is incomplete, run:");
    eprintln!("  claude plugin marketplace add {MARKETPLACE_SOURCE}");
    eprintln!("  claude plugin install {LARCH_ID}");
}

/// Move the active root back to `prior` after the new root failed verification.
///
/// The restore is a byte-identical rewrite of the pre-flip registry through the
/// confined atomic writer, followed by the same-surface check the upgrade uses:
/// `claude plugin list --json` must name `prior` again and that root's
/// executable must pass its bootstrap self-check.
fn recover_active_root(
    context: &Context,
    snapshot: Option<&RegistrySnapshot>,
    post_flip: Option<&RegistrySnapshot>,
    prior: &str,
    actual: &str,
) -> ActiveRootRecovery {
    if actual == prior {
        return ActiveRootRecovery::PointerUnchanged;
    }
    let Some(snapshot) = snapshot else {
        return ActiveRootRecovery::NoSnapshot;
    };
    eprintln!("Rolling the active larch plugin root back to {prior}...");
    if let Err(error) = context.restore_installed_registry(snapshot, post_flip) {
        return ActiveRootRecovery::RestoreFailed(error);
    }
    if context.installed_version().as_deref() != Some(prior) {
        return ActiveRootRecovery::RestoreUnverified;
    }
    let verified = context
        .resolve_installed_root(prior)
        .is_some_and(|root| context.bootstrap_installed_root(&root, prior));
    if verified {
        ActiveRootRecovery::RolledBack {
            prior: prior.to_owned(),
        }
    } else {
        ActiveRootRecovery::RestoreUnverified
    }
}

/// Print the post-flip state accurately and build the matching failure.
///
/// `unverified_root` is the cache root new sessions resolve while the pointer
/// still names the version whose executable never verified.
fn report_active_root_recovery(outcome: &ActiveRootRecovery, unverified_root: &Path) -> Failure {
    eprintln!();
    match outcome {
        ActiveRootRecovery::RolledBack { prior } => {
            eprintln!(
                "Rolled the active larch plugin root back to {prior}. New Claude Code sessions keep using it, and the running session is unaffected."
            );
            eprintln!("Recovery: retry /upgrade-larch.");
            return Failure::new(
                1,
                format!(
                    "Installed plugin verification failed; active root rolled back to {prior}."
                ),
            );
        }
        ActiveRootRecovery::PointerUnchanged => {
            eprintln!(
                "The install did not move the active plugin root to another version, so there is no earlier state to restore."
            );
        }
        ActiveRootRecovery::NoSnapshot => {
            eprintln!(
                "No pre-install registry snapshot exists, so the active plugin root could not be rolled back."
            );
        }
        ActiveRootRecovery::RestoreFailed(error) => {
            eprintln!("Rolling back the active plugin root failed: {error}");
        }
        ActiveRootRecovery::RestoreUnverified => {
            eprintln!(
                "The registry was restored, but claude plugin list --json and the prior root's executable did not confirm the prior version."
            );
        }
    }
    let root = unverified_root.display();
    eprintln!(
        "New Claude Code sessions resolve CLAUDE_PLUGIN_ROOT to {root}, which has no verified larch executable. Their fail-closed larch hooks deny Edit, Write, and Bash until it is installed. The running session is unaffected."
    );
    eprintln!(
        "Repair from a terminal outside Claude Code, with CLAUDE_PLUGIN_DATA set to an absolute, symlink-free directory:"
    );
    eprintln!(
        "  CLAUDE_PLUGIN_ROOT={root} CLAUDE_PLUGIN_DATA=<absolute-dir> {root}/scripts/larch.sh --version"
    );
    eprintln!("Then retry /upgrade-larch.");
    Failure::new(
        1,
        "Installed plugin verification failed; the active root was not rolled back.",
    )
}

fn unix_mode(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o777
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o600
    }
}
