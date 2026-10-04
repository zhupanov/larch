//! Genuine destination release bundles exercise the complete signature policy.

use larch_adapters::github::{
    AttestationFuture, AttestationOperations, AttestationQuery, AttestationServiceErrorKind,
    AttestationTransport,
};
use larch_core::{
    ArtifactAttestationRequest, ImmutableReleaseAttestationRequest, ReleaseAssetSubject,
    ReleaseSourceCommit, ReleaseTag,
};
use std::sync::Mutex;

const METADATA: &str = include_str!("fixtures/larch-destination-release.json");
const PROVENANCE: &[u8] = include_bytes!("fixtures/larch-destination-provenance.sigstore.json");
const RELEASE: &[u8] = include_bytes!("fixtures/larch-destination-release.sigstore.json");

struct Bundles(Mutex<Vec<Vec<u8>>>);

impl AttestationTransport for Bundles {
    fn fetch_bundles<'a>(&'a self, _query: &'a AttestationQuery) -> AttestationFuture<'a> {
        let value = std::mem::take(&mut *self.0.lock().expect("bundles"));
        Box::pin(async move { Ok(value) })
    }
}

fn run<T>(future: impl Future<Output = T>) -> T {
    larch_adapters::runtime::LarchRuntime::new()
        .expect("runtime")
        .block_on(future)
}

fn metadata() -> serde_json::Value {
    serde_json::from_str(METADATA).expect("captured release metadata")
}

fn tag() -> ReleaseTag {
    ReleaseTag::parse(metadata()["tag"].as_str().expect("tag")).expect("release tag")
}

fn commit() -> ReleaseSourceCommit {
    ReleaseSourceCommit::parse(metadata()["commit"].as_str().expect("commit"))
        .expect("release commit")
}

fn manifest() -> ReleaseAssetSubject {
    let captured = metadata();
    let item = captured["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .find(|asset| {
            asset["name"]
                .as_str()
                .expect("name")
                .ends_with("-manifest.json")
        })
        .expect("release manifest");
    ReleaseAssetSubject::new(
        item["name"].as_str().expect("name"),
        item["digest"].as_str().expect("digest"),
    )
    .expect("manifest subject")
}

fn artifact_request() -> ArtifactAttestationRequest {
    ArtifactAttestationRequest::new(manifest(), tag(), commit())
}

fn release_request() -> ImmutableReleaseAttestationRequest {
    let captured = metadata();
    let assets = captured["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .map(|item| {
            ReleaseAssetSubject::new(
                item["name"].as_str().expect("name"),
                item["digest"].as_str().expect("digest"),
            )
            .expect("asset subject")
        })
        .collect();
    ImmutableReleaseAttestationRequest::new(tag(), commit(), assets).expect("release request")
}

fn artifact_failure(
    bundles: Vec<Vec<u8>>,
    request: &ArtifactAttestationRequest,
) -> AttestationServiceErrorKind {
    let transport = Bundles(Mutex::new(bundles));
    run(AttestationOperations::new(&transport).verify_artifact(request))
        .expect_err("artifact refusal")
        .kind()
}

fn release_failure(
    bundles: Vec<Vec<u8>>,
    request: &ImmutableReleaseAttestationRequest,
) -> AttestationServiceErrorKind {
    let transport = Bundles(Mutex::new(bundles));
    run(AttestationOperations::new(&transport).verify_immutable_release(request))
        .expect_err("release refusal")
        .kind()
}

#[test]
fn accepts_real_destination_build_and_immutable_release_bundles() {
    let transport = Bundles(Mutex::new(vec![PROVENANCE.to_vec()]));
    run(AttestationOperations::new(&transport).verify_artifact(&artifact_request()))
        .expect("authentic destination build attestation");
    let transport = Bundles(Mutex::new(vec![RELEASE.to_vec()]));
    run(AttestationOperations::new(&transport).verify_immutable_release(&release_request()))
        .expect("authentic destination immutable release attestation");
}

#[test]
fn rejects_wrong_digest_commit_and_tag_from_otherwise_valid_destination_bundle() {
    let wrong_digest = ArtifactAttestationRequest::new(
        ReleaseAssetSubject::new(
            manifest().name(),
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("wrong digest"),
        tag(),
        commit(),
    );
    let wrong_commit = ArtifactAttestationRequest::new(
        manifest(),
        tag(),
        ReleaseSourceCommit::parse("1111111111111111111111111111111111111111")
            .expect("wrong commit"),
    );
    let wrong_tag = ArtifactAttestationRequest::new(
        manifest(),
        ReleaseTag::parse("v99.0.0").expect("wrong tag"),
        commit(),
    );
    for request in [wrong_digest, wrong_commit, wrong_tag] {
        assert_eq!(
            artifact_failure(vec![PROVENANCE.to_vec()], &request),
            AttestationServiceErrorKind::Verification
        );
    }
}

#[test]
fn rejects_incomplete_release_subjects_and_wrong_predicate() {
    let mut assets = release_request().assets().to_vec();
    let _ = assets.pop();
    let incomplete = ImmutableReleaseAttestationRequest::new(tag(), commit(), assets)
        .expect("bounded incomplete request");
    assert_eq!(
        release_failure(vec![RELEASE.to_vec()], &incomplete),
        AttestationServiceErrorKind::Verification
    );
    assert_eq!(
        artifact_failure(vec![RELEASE.to_vec()], &artifact_request()),
        AttestationServiceErrorKind::Verification
    );
    assert_eq!(
        release_failure(vec![PROVENANCE.to_vec()], &release_request()),
        AttestationServiceErrorKind::Verification
    );
}

#[test]
fn rejects_broken_signature_certificate_transparency_and_duplicate_bundles() {
    let mut certificate: serde_json::Value = serde_json::from_slice(PROVENANCE).expect("bundle");
    certificate["verificationMaterial"]["certificate"]["rawBytes"] = "AA==".into();
    let mut transparency: serde_json::Value = serde_json::from_slice(PROVENANCE).expect("bundle");
    transparency["verificationMaterial"]["tlogEntries"] = serde_json::json!([]);
    let mut signature: serde_json::Value = serde_json::from_slice(PROVENANCE).expect("bundle");
    signature["dsseEnvelope"]["signatures"][0]["sig"] = "AA==".into();
    for changed in [certificate, transparency, signature] {
        assert_eq!(
            artifact_failure(
                vec![serde_json::to_vec(&changed).expect("changed bundle")],
                &artifact_request(),
            ),
            AttestationServiceErrorKind::Verification
        );
    }
    assert_eq!(
        artifact_failure(
            vec![PROVENANCE.to_vec(), PROVENANCE.to_vec()],
            &artifact_request(),
        ),
        AttestationServiceErrorKind::Verification
    );
    assert_eq!(
        release_failure(vec![RELEASE.to_vec(), RELEASE.to_vec()], &release_request()),
        AttestationServiceErrorKind::Verification
    );
}
