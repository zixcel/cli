# Reviewed native provider host

A Rust library for vendor-owned, source-reviewed read adapters. It reuses Zix's
manifest/argument checks and Crowsi's transport/process boundaries. No provider is
automatically enabled because it exists in a specification.

Embed `zix-api-routes/v1` facts and a `zix-native-policy/v1` policy. The policy pins
the exact route bytes, package version, namespace, credential environment,
origin/base path, enabled API IDs, string argument rules, query/header constants,
response identity checks and scalar projections. Only reviewed GET/read methods
are supported by the data-driven HTTPS adapter. Candidate servers must match the
owner's separately declared origin and base path. Credentials come only from the
explicit selected environment variable; no SDK/CLI login, proxy, redirect, retry,
API activation or permission grant occurs. Read is not a claim of zero cost.

`entry(routes, policy, version)` implements `--manifest` and `--zix-provider-v1`.
It validates input before acquiring credentials, performs one bounded request,
checks identity, projects declared fields and emits one bounded response.
Results say single-response and do not infer IAM or account-wide completeness.
The native runtime needs no Node or sibling source checkout.

`serve(manifest, handler)` lets an existing owner library reuse the same protocol
without duplicating its vendor implementation. The handler remains trusted code
and owns scope/credentials/transport deadlines and observation meaning; the host
validates known read commands, protocol limits, output bounds and credential
redaction. This is not a sandbox, a grant issuer, a write executor or a signing
broker. Never label credential-signing or mutations as reads to fit this protocol.

Write workflows in existing TypeScript/Crowsi packages retain their separate
approval path. This source-only crate has `publish=false`; source tests and GNU
builds are not proof of registry distribution, musl or live-account acceptance.

When an upstream crate omits its original license files, the build owner may
retain those exact files under `tools/native-release/notices/<name>-<version>`
with `NOTICE.source.json` recording the official commit, URLs and hashes.
Packaged upstream notices take precedence. Missing material still stops packing;
this fallback does not replace a license review or invent license text.

Manifest-v2 hosts attach an executable/catalog/version `binding` to protocol
requests. SDK 0.1.1 parses and verifies it against the actual running executable
before calling any handler. Missing/changed bindings are rejected. This is
integrity checking, not an identity grant or a substitute for operator approval.
A v1-only provider must receive a v1 request; a composing v2 adapter validates its
outer request and explicitly removes the binding before using a retained v1 profile.
