# Local setup: install, update and diagnose

The standalone [install.sh](../install/install.sh) installs **published binaries**,
not a source checkout or compiler. Its independent release is `installer/v0.1.0`.
It is also served, byte-for-byte, at <https://zixcel.com/install.sh> after site
publication. `zix-setup` is an installation helper; it does not add service verbs
to the core dispatcher or replace `zix extensions`.

## Supported host and prerequisites

- Linux x86_64 with **glibc 2.39+**, including Ubuntu 24.04+ on WSL2. The current
  GNU core package actually imports GLIBC_2.39 symbols. Do not describe all Linux,
  Alpine/musl, ARM, macOS or native Windows as supported by this setup.
- Bash, Python **3.12+**, CA certificates and curl for the first download. No Rust,
  Node, global `gh`, GitHub login or provider token is required for package setup.
- `libsecret-1.so.0` for the GNU core. Actual credential enrollment additionally
  needs an existing user-session D-Bus and an unlocked Secret Service collection.
  Installing a library or a keyring package does not establish that session.
- Internet access to GitHub release/API and Sigstore verification infrastructure;
  the optional model also uses Hugging Face. Proxy/TLS failures and GitHub public
  API rate limits stop setup; they do not justify provider permission changes.

On Ubuntu 24.04, prepare the OS dependencies as your usual login user:

```sh
sudo apt-get update
sudo apt-get install -y --no-install-recommends python3 curl ca-certificates libsecret-1-0 gnome-keyring dbus-user-session
```

Run setup as your login user, **without sudo**. It does not change system packages,
shell startup files, service credentials, DNS or tenant configuration.

## First installation

Download into a private directory, verify and inspect the script before running.
The website is the HTTPS bootstrap trust entrypoint: its checksum detects a bad
or inconsistent download, but is not independent proof of who published it.
The installer release also has GitHub CI provenance for independent inspection.

```sh
mkdir -p "$HOME/.local/share/zix/bootstrap"
chmod 700 "$HOME/.local/share/zix/bootstrap"
cd "$HOME/.local/share/zix/bootstrap"
curl --fail --location https://zixcel.com/install.sh -o install.sh
curl --fail --location https://zixcel.com/SHA256SUMS.installer -o SHA256SUMS.installer
sha256sum --check SHA256SUMS.installer
# Inspect install.sh, then run:
bash install.sh install --with github
export PATH="$HOME/.local/bin:$PATH"
zix-setup doctor
zix capabilities --namespace github --compact --format json
```

Add the PATH export to your own shell configuration if desired. A missing PATH
entry does not mean installation failed: use `~/.local/bin/zix` directly.
For a mirror-independent download, use the assets of the exact
[installer release](https://github.com/zixcel/cli/releases/tag/installer/v0.1.0).

Only the core and explicitly selected extensions are installed. Select multiple
providers with `--with domain,microsoft` or repeat `--with`. First registration
is enabled; an existing disabled registration stays disabled, including updates.
Use `zix extensions enable NAMESPACE` when you intend to enable it.

## Optional 0.5B example environment

```sh
bash install.sh install --with github --local-model
export PATH="$HOME/.local/bin:$PATH"
zix-local-model --version
```

This additionally downloads the official llama.cpp **b11913** CPU runtime and
Qwen2.5-0.5B-Instruct **q8_0**, pinned by SHA256 and model source revision. Keep
about 2 GB free during installation and enough memory for inference (a machine
with at least 4 GB RAM is recommended). No model is downloaded by default.
`zix-local-model` is an external runtime wrapper, **not a Zix provider command**.
It supplies the installed model path, runtime library path, CPU backend and two
threads. The sample still supplies its prompt, grammar and output/context limits.
Secrets and provider credentials are not inherited by the model process.

The model/runtime pair stays at the tested versions; “latest” selection applies
to released Zix packages and the private GitHub verification helper, not to
untested model substitutions. Model routing accuracy is separate from successful
installation. See [LLM operation boundaries](llm-operations.md).

## Update and pin

```sh
zix-setup update
# Add one provider:
zix-setup install --with domain
# Reproduce a specific published selection:
zix-setup install --with github --pin cli=0.7.0 --pin github=0.2.0
```

`update` discovers the newest stable **installer** release, then updates the core,
all supported registered extensions (including disabled ones), the verification
helper and an optional previously installed sample environment. It selects native
packages by tag family and exact asset, not a repository's unrelated library
“latest” release. Unknown third-party namespaces are left unchanged. Explicit
rollback additionally requires `--allow-downgrade`; old version trees remain.

Every package is checked against its checksum index, archive path/size policy,
full contents manifest, release identity/source/target, and GitHub provenance
bound to its owner, workflow, source digest and tag. No candidate executable runs
until those checks pass. Update uses `extensions replace` with the observed old
hashes, preserving enabled state. A stale registry observation stops the update.

Updates are **per package**, not a transaction across all providers. If a later
package fails, completed updates remain and other registrations stay intact.
Inspect the receipt and current registrations before retrying. There is no
automatic service request, permission grant, write replay or downgrade.

## Verification helper and bootstrap trust

Setup uses a **private** `gh` at `~/.local/share/zix/setup/bin/gh`; a global older
`gh` is not used or replaced. It bootstraps the official GitHub CLI 2.103.3 archive
using the SHA256 embedded in this released script, then checks the newest stable
GitHub CLI release. A newer helper must additionally pass provenance verification
using the seed. Public attestation bundles are fetched anonymously and passed
with `--bundle`, so no `gh auth login`, PAT or saved GitHub configuration is read.

The seed script/hash is the initial trusted input; it is not self-authenticating.
If provenance cannot be verified, installation stops instead of silently reducing
the check to a checksum. Zix provider tokens remain separate from setup.

## Diagnose, storage and removal

```sh
zix-setup doctor
zix extensions list --format json
```

`doctor` makes no network or credential-store reads. It reports core/extension
versions, whether PATH selects the managed executable, D-Bus session presence and
whether `org.freedesktop.secrets` is currently owned. Ownership is an observation,
not proof that a collection is unlocked or token access will work. On WSL or a
headless host, prepare the login-session Secret Service separately; there is no
plaintext fallback. See [credential setup](credentials.md).

- Entry points: `~/.local/bin/zix`, `zix-setup`, optional `zix-local-model`.
- Immutable core/provider packages: `~/.local/share/zix/packages/`.
- Setup/helper/model files and receipts: `~/.local/share/zix/setup/`.
- Registry: the existing Zix user configuration directory; credentials stay in
  the OS store. Neither registry nor credential values are copied into prompts.

Temporary downloads are removed on success or failure; installed versions,
receipts and previous packages stay for rollback. Remove old trees only after
checking all registry references and retained rollback requirements. Unregister
an extension with `zix extensions remove NAMESPACE`; this does not erase packages
or credentials. Removing the setup entrypoint does not uninstall providers.
