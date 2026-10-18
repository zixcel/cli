# Install native release packages

Use a private download/install directory. Choose the release version explicitly when repeating a verified installation; omitting the tag below selects the latest official release. Do not run a source checkout or install a compiler as a substitute for a missing asset.

```sh
install -d -m 700 "$HOME/.local/share/zix/downloads/cli" "$HOME/.local/share/zix/downloads/google"
gh release download --repo zixcel/cli --pattern '*-x86_64-unknown-linux-musl.tar.gz' --pattern SHA256SUMS --dir "$HOME/.local/share/zix/downloads/cli"
gh release download --repo zixcel/zixcel-google --pattern 'zixcel-google-admin-*-x86_64-unknown-linux-musl.tar.gz' --pattern SHA256SUMS --dir "$HOME/.local/share/zix/downloads/google"
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

Use `extensions disable google`, `extensions enable google`, or `extensions remove google` to manage registration. Removal changes the registry, not package files or Google credentials. Updating a package requires an explicit remove/add of reviewed replacement bytes.

## Inspect one Google service-account key

In Bash, supply a short-lived access token through a hidden prompt, never an argv flag, chat message, source file, or old private key. Obtain that token using an already approved authentication flow. This first release does not implement OAuth login and does not require a local gcloud installation.

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
