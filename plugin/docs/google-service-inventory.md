# Google Service Inventory

This ledger records the production Google and `gcloud` caller inventory. Its
completion columns retain the historical #7727 cutover evidence.

## Checked scope

The inventory was refreshed for #7815 after the #7843 service repair leaves landed.
The scan covered production Rust, skills, agents, hooks, scripts, and CI
configuration. It excluded documentation, fixtures, historical run logs, and
the generated `plugin/` projection from caller classification.

The scan found one production Google API service and no production `gcloud`
process launch. Other matches are controls or setup prose:

- `crates/larch-core/src/config.rs` owns the
  `GOOGLE_APPLICATION_CREDENTIALS` environment name.
- `crates/larch-adapters/src/process.rs` removes Google credential variables
  from child environments and verifies that neither `gh` nor `gcloud` is an
  inherited executable.
- Redaction code recognizes Google API key shapes.
- Installation documentation shows operator-run ADC setup checks. Those
  commands are not larch production callers.

## Operation ledger

| Service category | Current production operations | Production callers | Adapter parity | Consumer cutover | Python removal |
| --- | --- | --- | --- | --- | --- |
| Cloud Storage | Bucket-root list preflight; paginated object list; create-only upload; download; metadata | Rust `run-log storage-preflight`, lifecycle, and publication commands through `GoogleCloudStorage`. | Complete: `GoogleCloudStorage` implements the larch-owned `ObjectStore` port with `google-cloud-storage` | Complete | Complete: Rust owns the production workflow and service client. |
| `gcloud` CLI | None | None | Not applicable | Not applicable | Not applicable |

`crates/larch-adapters/src/google_auth.rs` remains the single Google credential
owner. `crates/larch-adapters/src/google_storage.rs` owns the official Storage
and Storage Control clients behind the credential-free `ObjectStore` port and
DTOs in `larch-core`. It requests the
`https://www.googleapis.com/auth/devstorage.read_write` scope. Operators need
`storage.objects.list`, `storage.objects.get`, and `storage.objects.create`; no
overwrite or delete permission is required. Create-only uploads use generation
match zero. The `service-ownership` rule in
`crates/larch-lint` keeps concrete Google clients, `googleapis.com` request
surfaces, and the `gcloud` CLI inside `crates/larch-adapters`; larch never shells
out to `gcloud`.

Before adding a Google operation, update this ledger with its production caller,
service, exact OAuth scopes, minimum IAM permissions, larch-owned port and DTOs,
official client crate, and implementation evidence. Add offline fake-credential
and fake-transport coverage before enabling the operation. Keep any live test
ignored by default, explicit opt-in, and credential-free in its output.
