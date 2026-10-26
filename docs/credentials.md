# Local credential custody

`zix credential` composes the published `crowsi-credential-broker = 0.1.2`.
Crowsi owns envelope validation, revision checks, Secret Service transport and
bound, expiring one-use leases. Zix owns terminal authorization, registered
operation selection and child environment injection. No agent enrollment socket,
authority-issued grant, identity assertion or device attestation is simulated.

The GNU Linux package enables `credential-store`; the existing static musl package
does not link GLib/libsecret and returns `credential_store_not_in_build`. Both
provide help and use the same extension registry and immutable provider binaries.
No provider SDK or manifest protocol update is required.

## Prepare the OS provider

Use the operating system's supported Secret Service and keyring UI to provision
an encrypted persistent collection and select a supported alias explicitly.
GNOME Keyring supports only the `default` alias; an arbitrary `crowsi` alias
is not portable. Select the intended collection as default in the OS keyring UI.
Configure the provider's unlock prompt in the local desktop session. The provider
owns passwords, encryption, backups and recovery. Zix neither creates a collection
nor accepts its master password. `session` is rejected. Secret Service does not
attest encryption: the owner must verify that the actual persistent collection is
encrypted and has a password, rather than treating the alias as evidence.

The GNU package requires OS libsecret-1, GLib/GIO, a session DBus and a configured
Secret Service (for example, GNOME Keyring). Run from the same authorized user
session. A missing collection, locked store or unavailable bus fails; there is no
file, memory-store or alternate-collection fallback. No automatic unlock occurs.

```sh
zix credential status --collection default
zix credential unlock --collection default
```

Unlock invokes the OS provider's UI. It does not authorize a DNS write.
`lock` and `unlock` affect the whole selected collection. Do not select a shared
collection without considering its other consumers.

## Register an API token locally

References are `TENANT/SERVICE/PURPOSE/AUDIENCE/ID`, each component using ASCII
letters, digits, hyphens or underscores, with a maximum of 64 bytes. `AUDIENCE`
must equal the provider namespace at use time. The initial token interface accepts
nonempty printable ASCII values up to 16 KiB; it is not a binary-secret importer.

```sh
zix credential set --collection default \
  --reference local/valuedomain/api/domain/default \
  --host api-one.value-domain.com
```

This requires a real terminal, shows the reference/host and whether this is a
replacement, and asks for `yes`. The next prompt hides token input, restores
terminal settings on ordinary errors or Ctrl-C, and enforces the input bound
while reading. No secret argument, input file or plaintext export is accepted.
Replacement generates a new Crowsi revision; concurrent cross-process changes
are not a transactional CAS, so do not run concurrent enrollment/rotation.

```sh
zix credential inspect --collection default \
  --reference local/valuedomain/api/domain/default
```

Inspection emits only reference, revision and allowed hosts. Crowsi verifies the
complete stored envelope before returning metadata, so inspection requires an
unlocked store. There is no raw `get`, credential list or export command.

## Use with a registered extension

Inspect the registration and review the immutable manifest and executable hashes:

```sh
zix extensions inspect domain --format json
zix credential run --collection default \
  --reference local/valuedomain/api/domain/default \
  --host api-one.value-domain.com \
  --environment ZIX_VALUEDOMAIN_ACCESS_TOKEN \
  --manifest-sha256 REVIEWED_MANIFEST_SHA256 \
  --executable-sha256 REVIEWED_EXECUTABLE_SHA256 \
  -- domain list
```

Run accepts only a registered, enabled native provider operation, not an arbitrary
program, shell, `api` operation or nested credential command. It verifies the
reviewed hashes, audience, command-scoped environment declaration and absence of
an inherited value for that credential. Executable sealing and any operator-write
approval happen before reading the secret. Crowsi issues and consumes one 30-second
lease bound to the local invocation, executable, manifest, operation and arguments;
the host must match the stored allowed host. A provider update requires reviewing
new hashes. One invocation supplies one stored credential; operations requiring
multiple stored credentials are not supported by this interface.

Only the selected child's declared environment receives the value. Zix never
mutates its own process environment or writes a token cache. Plain `zix domain
list` continues to use the existing environment interface; it does not silently
read custody. Secret values and redaction copies owned by this composition are
zeroized on drop; OS process environment copies and dependency internals are not
claimed to be memory-nonexportable. Provider stderr is suppressed, and matching
plain or JSON-escaped secrets in responses are rejected by the existing dispatcher.

This is same-user local-owner authorization, suitable for an explicitly requested
chat-driven invocation after local enrollment. It is not isolation from root or
malicious code running as that same user. The local SPIFFE-shaped workload name
is a binding identifier, not proof of an authenticated workload. Allowed hosts are
lease constraints, not an OS network sandbox: the reviewed extension still owns
HTTP destinations, redirects and transport security. A trusted native extension
can read its supplied environment. All existing DNS plan and terminal approval
requirements remain in force when running an operator-write command.

## Rotation, removal and failure

`set` replaces a local credential only after terminal confirmation. It does not
create or rotate the remote API token. To remove the local entry:

```sh
zix credential remove --collection default \
  --reference local/valuedomain/api/domain/default
zix credential lock --collection default
```

Removal requires terminal confirmation and does not revoke the remote token.
Provider timeout/read-back failure can leave a mutation's outcome uncertain;
inspect through a new invocation before attempting recovery. There are no automatic
write retries. Unlock prompts and real token registration belong to the owner.

## Validation and distribution

Tests cover invalid input, secret flags, session collection refusal, terminal
requirements, target hashes, audiences, environment scope, disabled registrations
and inherited-value conflicts before store access. `tests/credential_os.py`
uses only synthetic secrets in an isolated DBus/XDG tree with an actual encrypted
GNOME keyring. It verifies enrollment, bound provider execution, secret output
rejection, restart persistence, locking, deletion and an unavailable provider.
It never connects to the caller's desktop bus or a live provider account.

The release workflow builds static musl and GNU custody packages independently.
`SHA256SUMS` continues to cover the musl archive for existing installers;
`SHA256SUMS.gnu` covers the GNU archive. Each package records its source, target,
binary hash and enabled features in `RELEASE.json`, retains dependency notices,
and has an attested archive. Install official release bytes, not `target/` binaries.

Design reviewed with Coela engineering catalog 0.2.4, source
`9f7673437d4d240a03b34b2501fc4807a14df721`, Japanese principles SHA256
`13bd69a229106ea3e040c445aae03b2aded3649a04703bda16ae555594110f48` and bundled
Rust resource/failure guidance 0.1.3. Source review, synthetic acceptance,
published artifacts, local provider readiness and live-account use are separate
evidence scopes; one does not prove the others.
