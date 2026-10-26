# Install native release packages

Use a private download/install directory. Choose the release version explicitly when repeating a verified installation; omitting the tag below selects the latest official release. Do not run a source checkout or install a compiler as a substitute for a missing asset.

```sh
install -d -m 700 "$HOME/.local/share/zix/downloads/cli" "$HOME/.local/share/zix/downloads/google"
gh release download --repo zixcel/cli --pattern '*-x86_64-unknown-linux-musl.tar.gz' --pattern SHA256SUMS --dir "$HOME/.local/share/zix/downloads/cli"
gh release download --repo zixcel/zixcel-google --pattern 'zixcel-google-cli-*-x86_64-unknown-linux-musl.tar.gz' --pattern SHA256SUMS --dir "$HOME/.local/share/zix/downloads/google"
(cd "$HOME/.local/share/zix/downloads/cli" && sha256sum --check SHA256SUMS)
(cd "$HOME/.local/share/zix/downloads/google" && sha256sum --check SHA256SUMS)
```

The release workflow records the exact source and archive hash in its accepted build and GitHub Release. With a GitHub CLI supporting artifact attestations, additionally verify each archive using `gh attestation verify ARCHIVE --repo OWNER/REPOSITORY`. Older CLI versions may lack that command: do not describe checksum-only inspection as cryptographic provenance verification.

Extract each verified archive in its download directory and verify its `CONTENTS.sha256`. Select the single extracted CLI directory as `ZIX_PACKAGE` and the single Google directory as `GOOGLE_PACKAGE`, both absolute paths. Keep them outside any group/world-writable directory except a root-owned sticky temporary ancestor.

```sh
(cd "$ZIX_PACKAGE" && sha256sum --check CONTENTS.sha256)
(cd "$GOOGLE_PACKAGE" && sha256sum --check CONTENTS.sha256)
"$ZIX_PACKAGE/zix" extensions add   --manifest "$GOOGLE_PACKAGE/google.extension.json"   --manifest-sha256 "$(sha256sum "$GOOGLE_PACKAGE/google.extension.json" | cut -d' ' -f1)"   --executable "$GOOGLE_PACKAGE/zix-google"   --executable-sha256 "$(sha256sum "$GOOGLE_PACKAGE/zix-google" | cut -d' ' -f1)"
```

Use `extensions disable google`, `extensions enable google`, or `extensions remove google` to manage registration. Removal changes the registry, not package files or Google credentials. Use `extensions list --format json` to inspect all registrations, including disabled ones.
`extensions inspect google --format json` returns the selected registration's enabled
state, package/version, exact hashes, executable path and manifest without executing
provider code. Missing binaries remain inspectable; capabilities and provider execution
still require enabled registrations.

With zix >=0.2.5, update or roll back by validating and replacing one registration
under one lock. Do not remove the old registration first:

```sh
zix extensions inspect google --format json > /absolute/private/google-before.json
zix extensions replace --manifest "$GOOGLE_PACKAGE/google.extension.json" \
  --manifest-sha256 REVIEWED_NEW_MANIFEST_SHA256 \
  --executable "$GOOGLE_PACKAGE/zix-google" \
  --executable-sha256 REVIEWED_NEW_EXECUTABLE_SHA256 \
  --expected-manifest-sha256 OBSERVED_OLD_MANIFEST_SHA256 \
  --expected-executable-sha256 OBSERVED_OLD_EXECUTABLE_SHA256
```

The expected hashes come from `.entry` in the inspection response. Stale observations,
namespace/package changes, incompatible manifests and invalid new files are rejected
before saving. Enabled/disabled state is preserved, including on a deliberate rollback.
The registry is atomically renamed and its directory synced. `registry_commit_unknown`
means inspect the registration after a post-rename durability failure; never blindly
restore a prior snapshot over another operator's work. An in-flight process keeps its
already verified executable; subsequent invocations see a complete old or new entry.

Keep the previous immutable package for rollback and use a fresh inspection with
`extensions replace` to select its reviewed hashes. Registration removal leaves files
and credentials intact. Remove unreferenced package directories only after checking
all registry references and any retained rollback evidence. No automatic downgrade,
latest-version selection, dependency solver or package activation is introduced.

## Inspect one Google service-account key

In Bash, supply a short-lived access token through a hidden prompt, never an argv flag, chat message, source file, or old private key. Obtain that token using an already approved authentication flow. The native extension does not implement OAuth login and does not require a local gcloud installation.

```sh
read -rsp 'Google access token: ' ZIX_GOOGLE_ACCESS_TOKEN
printf '\n'
export ZIX_GOOGLE_ACCESS_TOKEN
"$ZIX_PACKAGE/zix" google iam keys inspect   --project "$GOOGLE_PROJECT" --service-account "$GOOGLE_SERVICE_ACCOUNT" --key-id "$GOOGLE_KEY_ID"
unset ZIX_GOOGLE_ACCESS_TOKEN
```

The token needs the IAM or cloud-platform OAuth scope and IAM `iam.serviceAccounts.get` plus `iam.serviceAccountKeys.list` on the selected resource. A viewer role can supply these permissions; do not grant an editor/owner role just to run this observation.

`enabled`, `disabled`, `expired`, `not_yet_valid`, and `not_present` are distinct. `service_account_disabled` is reported separately. A key is `not_present` only after both account binding and a successful key-list response. HTTP 401/403/404, transport errors, rate limiting, invalid responses, scope mismatch and timeouts are failures, never revocation evidence. No key is used for login or modified.

Official API references: [service account get](https://docs.cloud.google.com/iam/docs/reference/rest/v1/projects.serviceAccounts/get), [key list](https://docs.cloud.google.com/iam/docs/reference/rest/v1/projects.serviceAccounts.keys/list).

Provider manifests can narrow environment inheritance per command with
`environment: {credential_environment: [], runtime_environment: []}`. Names in a
nonempty subset must already be allowed by that provider; duplicates and additions
are rejected. The declaration is part of the immutable manifest/binary selection,
not a caller-supplied runtime override. An omitted declaration keeps the existing
provider-wide requirements. Public read commands can therefore run without PATs
or DBus, while custody commands keep their explicit session requirements. This
requires zix 0.2.4 or newer; older dispatchers reject the unknown declaration.

## Linux credential-store package

For `zix credential`, select the GNU package from version 0.3.0 or newer. It uses
OS libsecret-1 and GLib/GIO, plus an existing session Secret Service. The static
musl package keeps its original dependency footprint and does not enable custody.
See [credential setup and scope](credentials.md) before registering a real token.

Download the GNU archive and its separate checksum file into a fresh private
version-specific directory (replace VERSION with an explicit published version):

```sh
gh release download vVERSION --repo zixcel/cli \
  --pattern '*-x86_64-unknown-linux-gnu.tar.gz' --pattern SHA256SUMS.gnu \
  --dir /absolute/private/download-directory
(cd /absolute/private/download-directory && sha256sum --check SHA256SUMS.gnu)
```

Verify the archive attestation when supported, extract, verify `CONTENTS.sha256`
and check `RELEASE.json` source, target and `features: ["credential-store"]`.
Select that immutable package's `zix` binary using the same local executable link;
the extensions registry and already installed provider packages need no migration.
Keep the previous official package for rollback. Switching back to static disables
custody commands without deleting OS credentials. Real OS collection provisioning
and unlock prompts are separate from the synthetic package acceptance.
