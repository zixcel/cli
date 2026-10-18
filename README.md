# Zixcel CLI (zix)

One local entry point for explicitly installed provider tools. Zix verifies a registered manifest and native executable, then dispatches a bounded JSON request through the published Crowsi process adapter. No provider is discovered from PATH or installed by running a command.

## Start

Download the Linux x86_64 musl archive from this repository's GitHub Releases and the Google extension from `zixcel/zixcel-google`. Verify their checksums and CI provenance before registering executable code. [Installation and Google IAM example](docs/install.md) explains the two independent packages.

```sh
zix list
zix google --help
zix describe google --format json
zix prompt google
```

The initial supported host is Linux x86_64, including WSL. The release binary does not require Node, Rust, gcloud, a sibling checkout, or a private registry. An unavailable release or provider is an error; there is no source-install or similarly named provider fallback.

## API specifications

Import selected OpenAPI or Google Discovery operations into a versioned catalog,
review upstream contract changes, and bind operations to installed providers.
[API architecture and update flow](docs/api-specifications.md) ·
[Offline playground](playground/api-catalog/README.md).

Catalogs do not execute arbitrary HTTP or grant permissions. Runtime bindings
require both the reviewed catalog hash and the registered provider manifest hash.

## Boundaries

- Extensions are trusted native executable code, not a sandbox. Review their origin and requested credential environment before enabling them.
- The first protocol supports declared read operations and string arguments. Write approval, Node/TypeScript launchers, Windows/macOS, and OAuth login/profile storage are not implemented by this release.
- Google access tokens are externally supplied, short-lived credentials. Zix does not mint a new token or authenticate with an old service-account key. The provider verifies Google resource scope; manifest permissions are descriptive, not authorization.
- A private per-user registry binds exact manifest/executable hashes. Namespace collisions, symlinks, unsafe ownership/modes, unknown protocol/operations, disabled providers and modified files fail closed.
- Before execution, verified ELF bytes are copied into a sealed Linux memfd. The selected file cannot be swapped between integrity verification and execution. Crowsi owns process lifetime; input, output, diagnostics and time are bounded. Unselected environment and raw provider diagnostics are not forwarded.

## Development

```sh
cargo test --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Release packaging selects only `--bin zix`. The contract probe and packaging executable are development tools and are not installed or shipped as runtime commands. Repository-local daily Dependabot proposes updates; it does not auto-merge or publish. [Release flow](docs/release.md).

Builds reuse the public Zixcel bounded-I/O component at the explicit Git revision in Cargo.toml and Cargo.lock. That managed component is not yet published to crates.io; no sibling checkout or private source is required. Runtime process ownership uses the published Crowsi process-adapter crate. Native packages include these components and do not fetch source when run.

Apache-2.0. Original dependency licenses and copyright notices accompany native archives in `THIRD-PARTY/`. `DEPENDENCIES.json` describes the resolved build/development graph, not a linked-binary SBOM.
