# Zixcel CLI (zix)

One local entry point for explicitly installed provider tools. Zix verifies a registered manifest and native executable, then dispatches a bounded JSON request through the published Crowsi process adapter. No provider is discovered from PATH or installed by running a command.

## Start

Download the dispatcher from this repository and selected provider extensions from
their owner releases. Use the GNU package for OS credential-store integration,
or the static package when custody is not needed. Verify checksums and CI
provenance before registration. [Installation](docs/install.md) includes an
independent provider example; [operation guidance](docs/operation-guidance.md)
explains Domain, Cloudflare and Microsoft handoffs.

```sh
zix list
zix google --help
zix describe google --format json
zix prompt google
zix extensions list --format json
zix extensions inspect google --format json
```

The initial supported host is Linux x86_64, including WSL. The release binary does not require Node, Rust, gcloud, a sibling checkout, or a private registry. An unavailable release or provider is an error; there is no source-install or similarly named provider fallback.

## Local credential custody

The GNU Linux package provides `zix credential` using the published Crowsi broker
and the OS Secret Service. Register tokens through a hidden local terminal prompt
and supply them only to a reviewed registered provider operation. There is no raw
secret export or plaintext fallback. The existing static package remains available.
[Credential setup, commands and trust boundaries](docs/credentials.md).

## API specifications

Import selected OpenAPI or Google Discovery operations into a versioned catalog,
review upstream contract changes, and bind operations to installed providers.
[API architecture and update flow](docs/api-specifications.md) ·
[Offline playground](playground/api-catalog/README.md) ·
[Native provider rollout](docs/provider-rollout.md).

API catalog commands are available in the 0.2 series. Explicit operator-write
manifests require 0.2.1 or later; the 0.2.0 release does not accept v2 write
manifests. Version 0.2.2 additionally distinguishes uncertain post-launch write
outcomes from ordinary read failures. A source change is not a published
release: check the owner release record before installing an extension.

Catalogs do not execute arbitrary HTTP or grant permissions. Runtime bindings
require both the reviewed catalog hash and the registered provider manifest hash.

## Boundaries

- Extensions are trusted native executable code, not a sandbox. Review their origin and requested credential environment before enabling them.
- V1 manifests declare read operations; opt-in v2 manifests support explicit local operator writes as documented below. Provider authentication and profiles belong to their owners; the dispatcher supplies scoped credential custody. Node/TypeScript launchers and Windows/macOS are not implemented.
- Google access tokens are externally supplied, short-lived credentials. Zix does not mint a new token or authenticate with an old service-account key. The provider verifies Google resource scope; manifest permissions are descriptive, not authorization.
- A private per-user registry binds exact manifest/executable hashes. Namespace collisions, symlinks, unsafe ownership/modes, unknown protocol/operations, disabled providers and modified files fail closed.
- Before execution, verified ELF bytes are copied into a sealed Linux memfd. The selected file cannot be swapped between integrity verification and execution. Crowsi owns process lifetime; input, output, diagnostics and time are bounded. Unselected environment and raw provider diagnostics are not forwarded.

## Development

```sh
cargo test --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Release packaging selects only `--bin zix`. The contract probe and packaging executable are development tools and are not installed or shipped as runtime commands. Repository-local daily Dependabot proposes updates; compatible signed updates
may merge through the configured owner CI gates. Package publication uses the
separate version-tag workflow. [Release flow](docs/release.md).

Builds reuse the public Zixcel bounded-I/O component at the explicit Git revision in Cargo.toml and Cargo.lock. That managed component is not yet published to crates.io; no sibling checkout or private source is required. Runtime process ownership uses the published Crowsi process-adapter crate. Native packages include these components and do not fetch source when run.

Apache-2.0. Original dependency licenses and copyright notices accompany native archives in `THIRD-PARTY/`. `DEPENDENCIES.json` describes the resolved build/development graph, not a linked-binary SBOM.

## Explicit local operator writes

The existing v1 manifest stays read-only. The opt-in v2 manifest may declare
`operator-write` commands with required plan-file, approval-sha256 and private
journal-directory arguments. The dispatcher requires a real interactive input
terminal unless `-y/--yes` is supplied, checks the plan's
operation/package/version/catalog/executable binding, and asks for `y/yes`
confirmation. An omitted approval hash is derived from the reviewed plan. Providers must independently
recheck the binding, current permissions/state, expiry, payload and durable journal.

This is an explicit local operator workflow, not a signed authorization grant or
isolation from a malicious same-user process/PTY. It does not implement remote
unattended write authority. Do not present it as the independent Crowsi authority
or use it to relax that separate boundary. A native API catalog still advertises
only the operations its own execution policy supports. Plan integrity contracts
are shared in the small `zix-operation-contracts` crate; provider API meaning stays
with each provider.

After a write child starts, timeout, cancellation or an invalid response cannot
prove that the external effect did not happen. The dispatcher reports
`operator_write_outcome_unknown` and never retries. Reconcile the provider receipt
and target state before planning a new operation. Detailed authentication/profile
and cancellation ownership is described in [provider rollout](docs/provider-rollout.md).

Version 0.2.5 adds registration inventory (including disabled extensions) and atomic,
expected-hash-bound `extensions replace` for upgrades and rollbacks. Provider catalogs
remain independently owned; no provider-specific command or package lookup is built
into the dispatcher. See [installation and lifecycle](docs/install.md).


## Write confirmation

Operator writes show the operation, plan path and requested change, then accept
`y` or `yes` at `Proceed? [y/N]`. Empty input and other answers cancel. Use `-y`
or `--yes` for explicit confirmation without a terminal, including in CI:

```sh
zix domain dns records apply --profile PROFILE --plan-file PLAN --journal-directory JOURNAL --yes
zix --yes microsoft exchange apply --tenant TENANT --subject SUBJECT --runtime-profile RUNTIME --plan-file PLAN --journal-directory JOURNAL
zix credential run --yes --collection COLLECTION --bindings BINDINGS --manifest-sha256 MANIFEST_HASH --executable-sha256 EXECUTABLE_HASH -- microsoft exchange apply --tenant TENANT --subject SUBJECT --runtime-profile RUNTIME --plan-file PLAN --journal-directory JOURNAL
```

Confirmation is limited to that invocation and is never inherited through an
environment variable. The dispatcher computes the plan's approval digest when
omitted. An explicitly supplied `--approval-sha256` remains an exact integrity
pin. File privacy, operation/catalog/version/executable binding, credentials,
provider expiry, fresh preflight and replay journals remain enforced. No automatic
retry follows an unknown write outcome. Existing provider packages retain their
protocol and do not need a new boolean argument in their manifests.

Credential mutation confirmations also accept `y`/`yes` or explicit `--yes`.
`credential set --yes` still reads the secret from a hidden terminal prompt;
confirmation flags never supply a secret or bypass the OS credential store.

## Operation meaning and diagnostics

CLI 0.6.0 accepts legacy provider protocol v1 and opt-in guided v2.
[Operation guidance](docs/operation-guidance.md) defines concise keyword summaries,
requirements, unresolved evidence and installed-tool handoffs. Provider-owned
catalogs generate discovery and runtime diagnostics from the same definitions.

For small-context agents, start with `zix capabilities --compact --format json`,
then inspect one operation using `zix describe NAMESPACE --operation ID --compact
--format json`. See [the LLM workflow](docs/llm-operations.md).
