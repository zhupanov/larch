use larch_adapters::github::{
    AttestationFuture, AttestationOperations, AttestationQuery, AttestationServiceError,
    AttestationServiceErrorKind, AttestationTransport,
};
use larch_core::{
    ArtifactAttestationRequest, ImmutableReleaseAttestationRequest, ReleaseAssetSubject,
    ReleaseSourceCommit, ReleaseTag,
};
use std::sync::Mutex;

const COMMIT: &str = "2a5825aa0b5be90d312f6334324c6ad147224265";
const MANIFEST_DIGEST: &str =
    "sha256:004234bde4081c977a2fd9c7eef3871b3f9e5253643445ad40c071fad84bdebf";
const PROVENANCE: &[u8] = include_bytes!("fixtures/larch-v53.1.24-provenance.sigstore.json");
const RELEASE: &[u8] = include_bytes!("fixtures/larch-v53.1.24-release.sigstore.json");

struct FakeTransport {
    responses: Mutex<Vec<Result<Vec<Vec<u8>>, AttestationServiceError>>>,
}

impl FakeTransport {
    fn bundles(bundles: Vec<Vec<u8>>) -> Self {
        Self {
            responses: Mutex::new(vec![Ok(bundles)]),
        }
    }

    fn error(error: AttestationServiceError) -> Self {
        Self {
            responses: Mutex::new(vec![Err(error)]),
        }
    }
}

impl AttestationTransport for FakeTransport {
    fn fetch_bundles<'a>(&'a self, _query: &'a AttestationQuery) -> AttestationFuture<'a> {
        let outcome = self.responses.lock().expect("responses").remove(0);
        Box::pin(async move { outcome })
    }
}

fn run<T>(future: impl Future<Output = T>) -> T {
    larch_adapters::runtime::LarchRuntime::new()
        .expect("runtime")
        .block_on(future)
}

fn tag() -> ReleaseTag {
    ReleaseTag::parse("v53.1.24").expect("tag")
}

fn commit() -> ReleaseSourceCommit {
    ReleaseSourceCommit::parse(COMMIT).expect("commit")
}

fn asset(name: &str, digest: &str) -> ReleaseAssetSubject {
    ReleaseAssetSubject::new(name, digest).expect("asset")
}

fn artifact_request() -> ArtifactAttestationRequest {
    ArtifactAttestationRequest::new(
        asset("larch-v53.1.24-manifest.json", MANIFEST_DIGEST),
        tag(),
        commit(),
    )
}

fn artifact_failure(
    bundles: Vec<Vec<u8>>,
    request: &ArtifactAttestationRequest,
) -> AttestationServiceErrorKind {
    let transport = FakeTransport::bundles(bundles);
    run(AttestationOperations::new(&transport).verify_artifact(request))
        .expect_err("artifact failure")
        .kind()
}

fn release_failure(
    bundles: Vec<Vec<u8>>,
    request: &ImmutableReleaseAttestationRequest,
) -> AttestationServiceErrorKind {
    let transport = FakeTransport::bundles(bundles);
    run(AttestationOperations::new(&transport).verify_immutable_release(request))
        .expect_err("release failure")
        .kind()
}

fn release_request() -> ImmutableReleaseAttestationRequest {
    ImmutableReleaseAttestationRequest::new(
        tag(),
        commit(),
        vec![
            asset(
                "larch-v53.1.24-aarch64-apple-darwin.tar.gz",
                "sha256:ff20bcb55dbb45acbeae6ea7b86f3c8257a264babb494f273c568f27d134dee6",
            ),
            asset(
                "larch-v53.1.24-aarch64-unknown-linux-gnu.tar.gz",
                "sha256:587febb69b0e968600ff083fd7eadd9699e946554e82fbbb9b2fb3cdf9235351",
            ),
            asset("larch-v53.1.24-manifest.json", MANIFEST_DIGEST),
            asset(
                "larch-v53.1.24-SHA256SUMS",
                "sha256:8004573cc8881aa363ccfb13a0025a946b775f65a3775d15676b730fde3b0b98",
            ),
            asset(
                "larch-v53.1.24-x86_64-apple-darwin.tar.gz",
                "sha256:df9780cfd5132831b3b08f8d2b8d63c725eb3a01e4381a2d6def8686c0b80ac8",
            ),
            asset(
                "larch-v53.1.24-x86_64-unknown-linux-gnu.tar.gz",
                "sha256:54b0079b7f1356e9debdc5527866c8ae3d35c8565980ae1e0984c0a591d13343",
            ),
        ],
    )
    .expect("release request")
}

#[test]
fn rejects_real_upstream_bundles_under_destination_repository_policy() {
    assert_eq!(
        artifact_failure(vec![PROVENANCE.to_vec()], &artifact_request()),
        AttestationServiceErrorKind::Verification
    );
    assert_eq!(
        release_failure(vec![RELEASE.to_vec()], &release_request()),
        AttestationServiceErrorKind::Verification
    );
}

#[test]
fn rejects_wrong_digest_commit_and_incomplete_release_asset_set() {
    let wrong_digest = ArtifactAttestationRequest::new(
        asset(
            "larch-v53.1.24-manifest.json",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        ),
        tag(),
        commit(),
    );
    assert_eq!(
        artifact_failure(vec![PROVENANCE.to_vec()], &wrong_digest),
        AttestationServiceErrorKind::Verification
    );

    let wrong_commit = ArtifactAttestationRequest::new(
        asset("larch-v53.1.24-manifest.json", MANIFEST_DIGEST),
        tag(),
        ReleaseSourceCommit::parse("1111111111111111111111111111111111111111").expect("commit"),
    );
    assert_eq!(
        artifact_failure(vec![PROVENANCE.to_vec()], &wrong_commit),
        AttestationServiceErrorKind::Verification
    );

    let wrong_identity = ArtifactAttestationRequest::new(
        asset("larch-v53.1.24-manifest.json", MANIFEST_DIGEST),
        ReleaseTag::parse("v53.1.25").expect("tag"),
        commit(),
    );
    assert_eq!(
        artifact_failure(vec![PROVENANCE.to_vec()], &wrong_identity),
        AttestationServiceErrorKind::Verification
    );

    assert_eq!(
        artifact_failure(vec![RELEASE.to_vec()], &artifact_request()),
        AttestationServiceErrorKind::Verification
    );

    let mut incomplete_assets = release_request().assets().to_vec();
    let _ = incomplete_assets.pop();
    let incomplete = ImmutableReleaseAttestationRequest::new(tag(), commit(), incomplete_assets)
        .expect("bounded incomplete request");
    assert_eq!(
        release_failure(vec![RELEASE.to_vec()], &incomplete),
        AttestationServiceErrorKind::Verification
    );
}

#[test]
fn rejects_malformed_certificates_missing_transparency_and_duplicate_matches() {
    let mut malformed: serde_json::Value = serde_json::from_slice(PROVENANCE).expect("fixture");
    malformed["verificationMaterial"]["certificate"]["rawBytes"] = "AA==".into();
    assert_eq!(
        artifact_failure(
            vec![serde_json::to_vec(&malformed).expect("json")],
            &artifact_request()
        ),
        AttestationServiceErrorKind::Verification
    );

    let mut no_transparency: serde_json::Value =
        serde_json::from_slice(PROVENANCE).expect("fixture");
    no_transparency["verificationMaterial"]["tlogEntries"] = serde_json::json!([]);
    assert_eq!(
        artifact_failure(
            vec![serde_json::to_vec(&no_transparency).expect("json")],
            &artifact_request()
        ),
        AttestationServiceErrorKind::Verification
    );

    assert_eq!(
        artifact_failure(
            vec![PROVENANCE.to_vec(), PROVENANCE.to_vec()],
            &artifact_request()
        ),
        AttestationServiceErrorKind::Verification
    );
}

#[test]
fn preserves_bounded_rate_limit_cancellation_timeout_and_size_classes() {
    for kind in [
        AttestationServiceErrorKind::RateLimited,
        AttestationServiceErrorKind::Cancelled,
        AttestationServiceErrorKind::DeadlineExceeded,
    ] {
        let error = AttestationServiceError::new(kind, None);
        let transport = FakeTransport::error(error);
        assert_eq!(
            run(AttestationOperations::new(&transport).verify_artifact(&artifact_request()))
                .expect_err("transport class")
                .kind(),
            kind
        );
    }

    assert_eq!(
        artifact_failure(vec![vec![0_u8; 2 * 1024 * 1024 + 1]], &artifact_request()),
        AttestationServiceErrorKind::LimitExceeded
    );
}
