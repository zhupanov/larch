//! GitHub artifact and immutable-release attestation verification.
//!
//! This adapter owns the fixed GitHub endpoints, bounded bundle retrieval,
//! Sigstore verification, certificate-extension policy, and statement policy.
//! Callers provide only validated larch release identities and cannot replace
//! the repository, workflow, issuer, signer identity, host, or trust root.

use super::{GitHubCompletionError, OctocrabGitHubService, collect_bounded_response};
use larch_core::{
    ArtifactAttestationRequest, ImmutableReleaseAttestationRequest, ProcessCancellation, SafeText,
    VerifiedArtifactAttestation, VerifiedReleaseAttestation,
};
use serde::Deserialize;
use sigstore_trust_root::SigstoreInstance;
use sigstore_types::{Bundle, Sha256Hash, SignatureContent};
use sigstore_verify::{VerificationPolicy, Verifier};
use snap::raw::{Decoder as SnappyDecoder, decompress_len};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
};
use url::Url;
use x509_cert::{Certificate, der::Decode};

const REPOSITORY: &str = "zhupanov/larch";
const REPOSITORY_URL: &str = "https://github.com/zhupanov/larch";
const WORKFLOW_PATH: &str = ".github/workflows/rust-release-assets.yaml";
const OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";
const GITHUB_HOSTED: &str = "github-hosted";
const RELEASE_IDENTITY: &str = "https://dotcom.releases.github.com";
const PROVENANCE_TYPE: &str = "https://slsa.dev/provenance/v1";
const RELEASE_TYPE: &str = "https://in-toto.io/attestation/release/v0.2";
const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
const WORKFLOW_BUILD_TYPE: &str = "https://actions.github.io/buildtypes/workflow/v1";
const MAX_BUNDLES: usize = 100;
const MAX_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;
const BUNDLE_HOST: &str = "tmaproduction.blob.core.windows.net";

/// Stable failure classes that never retain a bundle, certificate path, or URL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationServiceErrorKind {
    NotFound,
    Permission,
    RateLimited,
    MalformedResponse,
    LimitExceeded,
    Redirect,
    Verification,
    Cancelled,
    DeadlineExceeded,
    Transport,
}

/// A bounded and redacted attestation diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationServiceError {
    kind: AttestationServiceErrorKind,
    status: Option<u16>,
    detail: SafeText,
}

impl AttestationServiceError {
    /// Construct a transport-seam error without caller-supplied diagnostic text.
    #[must_use]
    pub fn new(kind: AttestationServiceErrorKind, status: Option<u16>) -> Self {
        let detail = match kind {
            AttestationServiceErrorKind::NotFound => "no matching GitHub attestation was found",
            AttestationServiceErrorKind::Permission => "GitHub attestation permission denied",
            AttestationServiceErrorKind::RateLimited => {
                "GitHub attestation request was rate limited"
            }
            AttestationServiceErrorKind::MalformedResponse => {
                "GitHub attestation response is malformed"
            }
            AttestationServiceErrorKind::LimitExceeded => {
                "GitHub attestation bundle limit exceeded"
            }
            AttestationServiceErrorKind::Redirect => {
                "GitHub attestation bundle redirect is not allowed"
            }
            AttestationServiceErrorKind::Verification => "GitHub attestation verification failed",
            AttestationServiceErrorKind::Cancelled => "GitHub attestation retrieval was cancelled",
            AttestationServiceErrorKind::DeadlineExceeded => {
                "GitHub attestation retrieval exceeded its deadline"
            }
            AttestationServiceErrorKind::Transport => "GitHub attestation transport failed",
        };
        Self {
            kind,
            status,
            detail: SafeText::from_untrusted(detail),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> AttestationServiceErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn status(&self) -> Option<u16> {
        self.status
    }
}

impl fmt::Display for AttestationServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.detail.fmt(formatter)
    }
}

impl Error for AttestationServiceError {}

/// A fixed, adapter-created query for one attestation class.
pub struct AttestationQuery {
    digest: String,
    predicate: &'static str,
    initiator: Option<&'static str>,
}

impl AttestationQuery {
    fn artifact(request: &ArtifactAttestationRequest) -> Self {
        Self {
            digest: request.subject().digest().to_owned(),
            predicate: "provenance",
            initiator: None,
        }
    }

    fn release(request: &ImmutableReleaseAttestationRequest) -> Self {
        Self {
            digest: request.source_commit().digest_with_algorithm(),
            predicate: "release",
            initiator: Some("github"),
        }
    }

    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    #[must_use]
    pub const fn predicate(&self) -> &'static str {
        self.predicate
    }

    #[must_use]
    pub const fn initiator(&self) -> Option<&'static str> {
        self.initiator
    }
}

/// Future returned by the attestation transport seam.
pub type AttestationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<Vec<u8>>, AttestationServiceError>> + Send + 'a>>;

/// Injectable retrieval seam. It returns bounded, decompressed bundle JSON.
pub trait AttestationTransport: Send + Sync {
    fn fetch_bundles<'a>(&'a self, query: &'a AttestationQuery) -> AttestationFuture<'a>;
}

/// Typed attestation verification over an injected retrieval transport.
pub struct AttestationOperations<'a, T: AttestationTransport> {
    transport: &'a T,
}

impl<'a, T: AttestationTransport> AttestationOperations<'a, T> {
    #[must_use]
    pub const fn new(transport: &'a T) -> Self {
        Self { transport }
    }

    /// Verify one artifact's build provenance against larch's fixed policy.
    ///
    /// # Errors
    /// Fails closed on retrieval, parsing, cryptographic, identity, or
    /// provenance-policy failure.
    pub async fn verify_artifact(
        &self,
        request: &ArtifactAttestationRequest,
    ) -> Result<VerifiedArtifactAttestation, AttestationServiceError> {
        let bundles = self
            .bounded_bundles(&AttestationQuery::artifact(request))
            .await?;
        let mut matches = 0_usize;
        for raw in bundles {
            if verify_artifact_bundle(&raw, request).is_ok() {
                matches += 1;
            }
        }
        require_single_match(matches)?;
        Ok(VerifiedArtifactAttestation {
            subject: request.subject().clone(),
            tag: request.tag().clone(),
            source_commit: request.source_commit().clone(),
        })
    }

    /// Verify GitHub's immutable-release attestation against the final asset set.
    ///
    /// # Errors
    /// Fails closed unless exactly one signed statement binds the expected tag,
    /// source commit, repository, and complete asset set.
    pub async fn verify_immutable_release(
        &self,
        request: &ImmutableReleaseAttestationRequest,
    ) -> Result<VerifiedReleaseAttestation, AttestationServiceError> {
        let bundles = self
            .bounded_bundles(&AttestationQuery::release(request))
            .await?;
        let mut matches = 0_usize;
        for raw in bundles {
            if verify_release_bundle(&raw, request).is_ok() {
                matches += 1;
            }
        }
        require_single_match(matches)?;
        Ok(VerifiedReleaseAttestation {
            tag: request.tag().clone(),
            source_commit: request.source_commit().clone(),
            asset_count: request.assets().len(),
        })
    }

    async fn bounded_bundles(
        &self,
        query: &AttestationQuery,
    ) -> Result<Vec<Vec<u8>>, AttestationServiceError> {
        let bundles = self.transport.fetch_bundles(query).await?;
        if bundles.is_empty() {
            return Err(failure(AttestationServiceErrorKind::NotFound));
        }
        if bundles.len() > MAX_BUNDLES
            || bundles.iter().any(|bundle| bundle.len() > MAX_BUNDLE_BYTES)
        {
            return Err(limit_failure());
        }
        Ok(bundles)
    }
}

fn require_single_match(matches: usize) -> Result<(), AttestationServiceError> {
    match matches {
        1 => Ok(()),
        0 | 2.. => Err(verification_failure()),
    }
}

fn verify_artifact_bundle(
    raw: &[u8],
    request: &ArtifactAttestationRequest,
) -> Result<(), AttestationServiceError> {
    let bundle = parse_bundle(raw)?;
    let identity = workflow_identity(&request.tag().source_ref());
    let policy = VerificationPolicy::with_identity(identity.clone()).require_issuer(OIDC_ISSUER);
    let root = SigstoreInstance::PublicGood
        .embedded_trusted_root()
        .map_err(|_| verification_failure())?;
    let digest =
        Sha256Hash::from_hex(request.subject().digest_hex()).map_err(|_| verification_failure())?;
    Verifier::new(&root)
        .verify(digest, &bundle, &policy)
        .map_err(|_| verification_failure())?;
    validate_certificate_extensions(
        &bundle,
        &identity,
        request.source_commit().as_str(),
        &request.tag().source_ref(),
    )?;
    let statement = statement(&bundle)?;
    validate_artifact_statement(&statement, request)
}

fn verify_release_bundle(
    raw: &[u8],
    request: &ImmutableReleaseAttestationRequest,
) -> Result<(), AttestationServiceError> {
    let bundle = parse_bundle(raw)?;
    let root = SigstoreInstance::GitHub
        .embedded_trusted_root()
        .map_err(|_| verification_failure())?;
    let digest = Sha256Hash::from_hex(request.assets()[0].digest_hex())
        .map_err(|_| verification_failure())?;
    let policy = VerificationPolicy::with_identity(RELEASE_IDENTITY)
        .skip_sct()
        .skip_tlog();
    Verifier::new(&root)
        .verify(digest, &bundle, &policy)
        .map_err(|_| verification_failure())?;
    let statement = statement(&bundle)?;
    validate_release_statement(&statement, request)
}

fn parse_bundle(raw: &[u8]) -> Result<Bundle, AttestationServiceError> {
    let text = std::str::from_utf8(raw).map_err(|_| malformed())?;
    Bundle::from_json(text).map_err(|_| malformed())
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Statement {
    #[serde(rename = "_type")]
    type_: String,
    subject: Vec<Subject>,
    predicate_type: String,
    predicate: serde_json::Value,
}

#[derive(Clone, Deserialize)]
struct Subject {
    #[serde(default)]
    name: String,
    #[serde(default)]
    uri: String,
    digest: BTreeMap<String, String>,
}

fn statement(bundle: &Bundle) -> Result<Statement, AttestationServiceError> {
    let SignatureContent::DsseEnvelope(envelope) = &bundle.content else {
        return Err(malformed());
    };
    if envelope.payload_type != "application/vnd.in-toto+json" {
        return Err(malformed());
    }
    serde_json::from_slice(envelope.payload.as_bytes()).map_err(|_| malformed())
}

fn validate_artifact_statement(
    statement: &Statement,
    request: &ArtifactAttestationRequest,
) -> Result<(), AttestationServiceError> {
    require_statement_header(statement, PROVENANCE_TYPE)?;
    require_unique_subjects(&statement.subject)?;
    let matched = statement.subject.iter().filter(|subject| {
        subject.name == request.subject().name()
            && subject.digest.len() == 1
            && subject.digest.get("sha256").map(String::as_str)
                == Some(request.subject().digest_hex())
    });
    if matched.count() != 1 {
        return Err(verification_failure());
    }
    let workflow = json_path(
        &statement.predicate,
        &["buildDefinition", "externalParameters", "workflow"],
    )?;
    require_json_string(workflow, "repository", REPOSITORY_URL)?;
    require_json_string(workflow, "path", WORKFLOW_PATH)?;
    require_json_string(workflow, "ref", &request.tag().source_ref())?;
    require_json_string(
        json_path(&statement.predicate, &["buildDefinition"])?,
        "buildType",
        WORKFLOW_BUILD_TYPE,
    )?;
    let github = json_path(
        &statement.predicate,
        &["buildDefinition", "internalParameters", "github"],
    )?;
    require_json_string(github, "runner_environment", GITHUB_HOSTED)?;
    let builder = json_path(&statement.predicate, &["runDetails", "builder"])?;
    require_json_string(
        builder,
        "id",
        &workflow_identity(&request.tag().source_ref()),
    )?;
    let dependencies = json_path(
        &statement.predicate,
        &["buildDefinition", "resolvedDependencies"],
    )?
    .as_array()
    .ok_or_else(verification_failure)?;
    let expected_uri = format!("git+{REPOSITORY_URL}@{}", request.tag().source_ref());
    let matching_dependencies = dependencies
        .iter()
        .filter(|dependency| {
            dependency.get("uri").and_then(serde_json::Value::as_str) == Some(expected_uri.as_str())
                && dependency
                    .get("digest")
                    .and_then(|value| value.get("gitCommit"))
                    .and_then(serde_json::Value::as_str)
                    == Some(request.source_commit().as_str())
        })
        .count();
    if matching_dependencies != 1 {
        return Err(verification_failure());
    }
    Ok(())
}

fn validate_release_statement(
    statement: &Statement,
    request: &ImmutableReleaseAttestationRequest,
) -> Result<(), AttestationServiceError> {
    require_statement_header(statement, RELEASE_TYPE)?;
    require_unique_subjects(&statement.subject)?;
    require_json_string(&statement.predicate, "repository", REPOSITORY)?;
    require_json_string(&statement.predicate, "tag", request.tag().as_str())?;
    let purl = format!("pkg:github/{REPOSITORY}@{}", request.tag().as_str());
    require_json_string(&statement.predicate, "purl", &purl)?;

    let commit_algorithm = if request.source_commit().as_str().len() == 40 {
        "sha1"
    } else {
        "sha256"
    };
    let release_subjects = statement
        .subject
        .iter()
        .filter(|subject| {
            subject.uri == purl
                && subject.name.is_empty()
                && subject.digest.len() == 1
                && subject.digest.get(commit_algorithm).map(String::as_str)
                    == Some(request.source_commit().as_str())
        })
        .count();
    if release_subjects != 1 {
        return Err(verification_failure());
    }

    let actual_assets: BTreeMap<&str, &str> = statement
        .subject
        .iter()
        .filter(|subject| !subject.name.is_empty())
        .map(|subject| {
            let digest = subject
                .digest
                .get("sha256")
                .filter(|_| subject.digest.len() == 1)
                .map(String::as_str)
                .ok_or_else(verification_failure)?;
            Ok((subject.name.as_str(), digest))
        })
        .collect::<Result<_, AttestationServiceError>>()?;
    let expected_assets: BTreeMap<&str, &str> = request
        .assets()
        .iter()
        .map(|asset| (asset.name(), asset.digest_hex()))
        .collect();
    if actual_assets != expected_assets || statement.subject.len() != expected_assets.len() + 1 {
        return Err(verification_failure());
    }
    Ok(())
}

fn require_statement_header(
    statement: &Statement,
    predicate_type: &str,
) -> Result<(), AttestationServiceError> {
    if statement.type_ == STATEMENT_TYPE && statement.predicate_type == predicate_type {
        Ok(())
    } else {
        Err(verification_failure())
    }
}

fn require_unique_subjects(subjects: &[Subject]) -> Result<(), AttestationServiceError> {
    if subjects.is_empty() {
        return Err(verification_failure());
    }
    let mut names = BTreeSet::new();
    let mut digests = BTreeSet::new();
    for subject in subjects {
        let name = if subject.name.is_empty() {
            subject.uri.as_str()
        } else {
            subject.name.as_str()
        };
        if name.is_empty() || !names.insert(name) {
            return Err(verification_failure());
        }
        for (algorithm, digest) in &subject.digest {
            if !digests.insert((algorithm.as_str(), digest.as_str())) {
                return Err(verification_failure());
            }
        }
    }
    Ok(())
}

fn json_path<'a>(
    root: &'a serde_json::Value,
    path: &[&str],
) -> Result<&'a serde_json::Value, AttestationServiceError> {
    let mut value = root;
    for segment in path {
        value = value.get(segment).ok_or_else(verification_failure)?;
    }
    Ok(value)
}

fn require_json_string(
    object: &serde_json::Value,
    key: &str,
    expected: &str,
) -> Result<(), AttestationServiceError> {
    if object.get(key).and_then(serde_json::Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(verification_failure())
    }
}

fn workflow_identity(source_ref: &str) -> String {
    format!("{REPOSITORY_URL}/{WORKFLOW_PATH}@{source_ref}")
}

fn validate_certificate_extensions(
    bundle: &Bundle,
    identity: &str,
    commit: &str,
    source_ref: &str,
) -> Result<(), AttestationServiceError> {
    let certificate = bundle
        .signing_certificate()
        .ok_or_else(verification_failure)?;
    let certificate =
        Certificate::from_der(certificate.as_bytes()).map_err(|_| verification_failure())?;
    let expected = [
        ("1.3.6.1.4.1.57264.1.9", identity),
        ("1.3.6.1.4.1.57264.1.10", commit),
        ("1.3.6.1.4.1.57264.1.11", GITHUB_HOSTED),
        ("1.3.6.1.4.1.57264.1.12", REPOSITORY_URL),
        ("1.3.6.1.4.1.57264.1.13", commit),
        ("1.3.6.1.4.1.57264.1.14", source_ref),
    ];
    let extensions = certificate
        .tbs_certificate
        .extensions
        .as_ref()
        .ok_or_else(verification_failure)?;
    for (oid, expected_value) in expected {
        let extension = extensions
            .iter()
            .find(|extension| extension.extn_id.to_string() == oid)
            .ok_or_else(verification_failure)?;
        let value = decode_extension(extension.extn_value.as_bytes())?;
        if value != expected_value {
            return Err(verification_failure());
        }
    }
    Ok(())
}

fn decode_extension(bytes: &[u8]) -> Result<String, AttestationServiceError> {
    if let Ok(value) = x509_cert::der::asn1::Utf8StringRef::from_der(bytes) {
        return Ok(value.to_string());
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| verification_failure())
}

#[derive(Deserialize)]
struct AttestationResponse {
    attestations: Vec<AttestationRecord>,
}

#[derive(Deserialize)]
struct AttestationRecord {
    #[serde(default)]
    bundle: Option<serde_json::Value>,
    #[serde(default)]
    bundle_url: Option<String>,
    initiator: String,
}

/// The raw-bytes source for one attestation record's bundle.
#[derive(Debug)]
enum BundleSource {
    /// Bytes inlined directly in the attestations API response.
    Inline(Vec<u8>),
    /// A validated attestation-store URL to fetch the bundle from.
    Fetch(String),
}

/// Choose a record's bundle source, preferring the inline `bundle` when the
/// API populates both `bundle` and `bundle_url`. Only a record with neither
/// field is malformed.
fn select_bundle_source(
    bundle: Option<serde_json::Value>,
    bundle_url: Option<String>,
) -> Result<BundleSource, AttestationServiceError> {
    match (bundle, bundle_url) {
        (Some(bundle), _) => Ok(BundleSource::Inline(
            serde_json::to_vec(&bundle).map_err(|_| malformed())?,
        )),
        (None, Some(url)) => Ok(BundleSource::Fetch(url)),
        (None, None) => Err(malformed()),
    }
}

/// Production retrieval through the authenticated, deadline-bound client.
pub struct OctocrabAttestationTransport<'a> {
    service: &'a OctocrabGitHubService,
    cancellation: &'a dyn ProcessCancellation,
}

impl<'a> OctocrabAttestationTransport<'a> {
    #[must_use]
    pub const fn new(
        service: &'a OctocrabGitHubService,
        cancellation: &'a dyn ProcessCancellation,
    ) -> Self {
        Self {
            service,
            cancellation,
        }
    }

    async fn guarded_fetch(
        &self,
        query: &AttestationQuery,
    ) -> Result<Vec<Vec<u8>>, AttestationServiceError> {
        match self
            .service
            .guard_operation(self.cancellation, self.fetch(query))
            .await
        {
            Ok(result) => result,
            Err(GitHubCompletionError::Cancelled) => {
                Err(failure(AttestationServiceErrorKind::Cancelled))
            }
            Err(GitHubCompletionError::DeadlineExceeded) => {
                Err(failure(AttestationServiceErrorKind::DeadlineExceeded))
            }
        }
    }

    async fn fetch(
        &self,
        query: &AttestationQuery,
    ) -> Result<Vec<Vec<u8>>, AttestationServiceError> {
        let url = format!(
            "https://api.github.com/repos/{REPOSITORY}/attestations/{}?predicate_type={}&per_page={MAX_BUNDLES}",
            query.digest(),
            query.predicate()
        );
        let response = self
            .service
            .client
            ._get(url)
            .await
            .map_err(|error| map_octocrab_error(&error))?;
        if has_next_page(response.headers()) {
            return Err(limit_failure());
        }
        let body = collect_bounded_response(response, MAX_BUNDLE_BYTES)
            .await
            .map_err(|()| limit_failure())?;
        let response: AttestationResponse =
            serde_json::from_slice(&body).map_err(|_| malformed())?;
        if response.attestations.len() > MAX_BUNDLES {
            return Err(limit_failure());
        }
        let mut bundles = Vec::with_capacity(response.attestations.len());
        for record in response.attestations {
            if query
                .initiator()
                .is_some_and(|expected| record.initiator != expected)
            {
                continue;
            }
            if record.initiator.is_empty() {
                return Err(malformed());
            }
            let raw = match select_bundle_source(record.bundle, record.bundle_url)? {
                BundleSource::Inline(bytes) => bytes,
                BundleSource::Fetch(url) => self.fetch_bundle_url(&url).await?,
            };
            if raw.len() > MAX_BUNDLE_BYTES {
                return Err(limit_failure());
            }
            bundles.push(raw);
        }
        Ok(bundles)
    }

    async fn fetch_bundle_url(&self, value: &str) -> Result<Vec<u8>, AttestationServiceError> {
        let mut current = validate_bundle_url(value, None)?;
        let mut visited = BTreeSet::new();
        for _ in 0..=MAX_REDIRECTS {
            if !visited.insert(current.as_str().to_owned()) {
                return Err(redirect_failure());
            }
            let request = http::Request::builder()
                .method(http::Method::GET)
                .uri(current.as_str())
                .header(http::header::ACCEPT, "application/octet-stream")
                .body(())
                .map_err(|_| malformed())?;
            let response = self
                .service
                .client
                .execute(request)
                .await
                .map_err(|error| map_octocrab_error(&error))?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(http::header::LOCATION)
                    .and_then(|header| header.to_str().ok())
                    .ok_or_else(redirect_failure)?;
                current = validate_bundle_url(location, Some(&current))?;
                continue;
            }
            if response.status() != http::StatusCode::OK {
                return Err(status_failure(response.status().as_u16()));
            }
            let compressed = collect_bounded_response(response, MAX_BUNDLE_BYTES)
                .await
                .map_err(|()| limit_failure())?;
            let size = decompress_len(&compressed).map_err(|_| malformed())?;
            if size > MAX_BUNDLE_BYTES {
                return Err(limit_failure());
            }
            return SnappyDecoder::new()
                .decompress_vec(&compressed)
                .map_err(|_| malformed());
        }
        Err(redirect_failure())
    }
}

impl AttestationTransport for OctocrabAttestationTransport<'_> {
    fn fetch_bundles<'a>(&'a self, query: &'a AttestationQuery) -> AttestationFuture<'a> {
        Box::pin(self.guarded_fetch(query))
    }
}

fn validate_bundle_url(value: &str, base: Option<&Url>) -> Result<Url, AttestationServiceError> {
    let url = base
        .map_or_else(|| Url::parse(value), |base| base.join(value))
        .map_err(|_| redirect_failure())?;
    let approved = url.scheme() == "https"
        && url.host_str() == Some(BUNDLE_HOST)
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && url.path().starts_with("/attestations/");
    if approved {
        Ok(url)
    } else {
        Err(redirect_failure())
    }
}

fn has_next_page(headers: &http::HeaderMap) -> bool {
    headers
        .get_all(http::header::LINK)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|link| {
            link.split(';')
                .skip(1)
                .any(|parameter| parameter.trim() == "rel=\"next\"")
        })
}

fn map_octocrab_error(error: &octocrab::Error) -> AttestationServiceError {
    let status = match error {
        octocrab::Error::GitHub { source, .. } => Some(source.status_code.as_u16()),
        _ => None,
    };
    let rate_limited = status == Some(403)
        && error
            .to_string()
            .to_ascii_lowercase()
            .contains("rate limit");
    let kind = match status {
        Some(403) if rate_limited => AttestationServiceErrorKind::RateLimited,
        Some(401 | 403) => AttestationServiceErrorKind::Permission,
        Some(404) => AttestationServiceErrorKind::NotFound,
        Some(429) => AttestationServiceErrorKind::RateLimited,
        _ => AttestationServiceErrorKind::Transport,
    };
    AttestationServiceError::new(kind, status)
}

fn status_failure(status: u16) -> AttestationServiceError {
    let kind = match status {
        401 | 403 => AttestationServiceErrorKind::Permission,
        404 => AttestationServiceErrorKind::NotFound,
        429 => AttestationServiceErrorKind::RateLimited,
        _ => AttestationServiceErrorKind::Transport,
    };
    AttestationServiceError::new(kind, Some(status))
}

fn failure(kind: AttestationServiceErrorKind) -> AttestationServiceError {
    AttestationServiceError::new(kind, None)
}

fn malformed() -> AttestationServiceError {
    failure(AttestationServiceErrorKind::MalformedResponse)
}

fn limit_failure() -> AttestationServiceError {
    failure(AttestationServiceErrorKind::LimitExceeded)
}

fn redirect_failure() -> AttestationServiceError {
    failure(AttestationServiceErrorKind::Redirect)
}

fn verification_failure() -> AttestationServiceError {
    failure(AttestationServiceErrorKind::Verification)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROVENANCE: &[u8] =
        include_bytes!("../../tests/fixtures/larch-v53.1.24-provenance.sigstore.json");

    fn artifact_request() -> ArtifactAttestationRequest {
        ArtifactAttestationRequest::new(
            larch_core::ReleaseAssetSubject::new(
                "larch-v53.1.24-manifest.json",
                "sha256:004234bde4081c977a2fd9c7eef3871b3f9e5253643445ad40c071fad84bdebf",
            )
            .expect("asset"),
            larch_core::ReleaseTag::parse("v53.1.24").expect("tag"),
            larch_core::ReleaseSourceCommit::parse("2a5825aa0b5be90d312f6334324c6ad147224265")
                .expect("commit"),
        )
    }

    fn provenance_statement() -> Statement {
        let mut value = statement(&parse_bundle(PROVENANCE).expect("bundle")).expect("statement");
        let source_ref = artifact_request().tag().source_ref();
        // Exercise destination statement policy without rewriting a signed fixture.
        value.predicate["buildDefinition"]["externalParameters"]["workflow"]["repository"] =
            REPOSITORY_URL.into();
        value.predicate["runDetails"]["builder"]["id"] = workflow_identity(&source_ref).into();
        value.predicate["buildDefinition"]["resolvedDependencies"][0]["uri"] =
            format!("git+{REPOSITORY_URL}@{source_ref}").into();
        value
    }

    #[test]
    fn upstream_provenance_is_not_destination_provenance() {
        let upstream = statement(&parse_bundle(PROVENANCE).expect("bundle")).expect("statement");
        assert_eq!(
            validate_artifact_statement(&upstream, &artifact_request())
                .expect_err("upstream repository is outside destination policy")
                .kind(),
            AttestationServiceErrorKind::Verification
        );
    }

    #[test]
    fn provenance_policy_rejects_every_identity_and_hosted_runner_mismatch() {
        let request = artifact_request();
        validate_artifact_statement(&provenance_statement(), &request)
            .expect("valid destination statement policy");

        let mut variants = Vec::new();
        for path in [
            &[
                "buildDefinition",
                "externalParameters",
                "workflow",
                "repository",
            ][..],
            &["buildDefinition", "externalParameters", "workflow", "path"][..],
            &["buildDefinition", "externalParameters", "workflow", "ref"][..],
            &[
                "buildDefinition",
                "internalParameters",
                "github",
                "runner_environment",
            ][..],
            &["runDetails", "builder", "id"][..],
        ] {
            let mut statement = provenance_statement();
            let mut value = &mut statement.predicate;
            for segment in path {
                value = value.get_mut(segment).expect("fixture path");
            }
            *value = serde_json::Value::String("wrong".to_owned());
            variants.push(statement);
        }
        let mut missing_repository = provenance_statement();
        missing_repository.predicate["buildDefinition"]["externalParameters"]["workflow"]
            .as_object_mut()
            .expect("workflow")
            .remove("repository");
        variants.push(missing_repository);
        let mut wrong_commit = provenance_statement();
        wrong_commit.predicate["buildDefinition"]["resolvedDependencies"][0]["digest"]["gitCommit"] =
            "1111111111111111111111111111111111111111".into();
        variants.push(wrong_commit);

        let mut duplicate = provenance_statement();
        duplicate.subject.push(duplicate.subject[0].clone());
        variants.push(duplicate);

        for variant in variants {
            assert_eq!(
                validate_artifact_statement(&variant, &request)
                    .expect_err("policy mismatch")
                    .kind(),
                AttestationServiceErrorKind::Verification
            );
        }
    }

    #[test]
    fn bundle_url_policy_allows_only_the_fixed_attestation_store() {
        let valid = validate_bundle_url(
            "https://tmaproduction.blob.core.windows.net/attestations/1/a.json.sn?sig=secret",
            None,
        )
        .expect("approved URL");
        assert!(validate_bundle_url("next.json.sn?sig=other", Some(&valid)).is_ok());
        for denied in [
            "http://tmaproduction.blob.core.windows.net/attestations/1/a",
            "https://user@tmaproduction.blob.core.windows.net/attestations/1/a",
            "https://productionresultssa1.blob.core.windows.net/attestations/1/a",
            "https://tmaproduction.blob.core.windows.net/other/1/a",
        ] {
            assert!(validate_bundle_url(denied, None).is_err(), "{denied}");
        }
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::LINK,
            "<https://api.github.com/next>; rel=\"next\""
                .parse()
                .expect("header"),
        );
        assert!(has_next_page(&headers));
        assert_eq!(
            status_failure(429).kind(),
            AttestationServiceErrorKind::RateLimited
        );
    }

    #[test]
    fn diagnostics_never_render_response_content_or_urls() {
        for error in [
            malformed(),
            limit_failure(),
            redirect_failure(),
            verification_failure(),
        ] {
            let rendered = error.to_string();
            assert!(!rendered.contains("blob.core"));
            assert!(!rendered.contains("certificate"));
            assert!(!rendered.contains("bundle content"));
            assert!(rendered.len() < 100);
        }
    }

    #[test]
    fn bundle_source_prefers_inline_bundle_when_both_are_present() {
        let bundle = serde_json::json!({
            "mediaType": "application/vnd.dev.sigstore.bundle+json;version=0.3"
        });
        let store_url =
            "https://tmaproduction.blob.core.windows.net/attestations/1/a.json.sn?sig=x".to_owned();

        match select_bundle_source(Some(bundle.clone()), Some(store_url.clone()))
            .expect("inline bundle wins")
        {
            BundleSource::Inline(bytes) => {
                assert_eq!(bytes, serde_json::to_vec(&bundle).expect("bytes"));
            }
            BundleSource::Fetch(_) => panic!("both present must prefer the inline bundle"),
        }

        match select_bundle_source(None, Some(store_url)).expect("url source") {
            BundleSource::Fetch(url) => assert!(url.contains("tmaproduction")),
            BundleSource::Inline(_) => panic!("url-only record must fetch"),
        }

        assert_eq!(
            select_bundle_source(None, None)
                .expect_err("record with neither field")
                .kind(),
            AttestationServiceErrorKind::MalformedResponse
        );
    }
}
