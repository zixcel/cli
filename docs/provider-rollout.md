# Provider rollout and independent ownership

The common entry remains `zixcel/cli` / `zix`. Shared API parsing, bounded native
protocol and reviewed read-profile hosting belong here. Generic HTTPS/process
execution belongs to Crowsi. Vendor policy and release acceptance belong to each
vendor owner; no central release action or credential is introduced.

| Owner | Specification / existing implementation | Native read scope |
| --- | --- | --- |
| zixcel-azure | Official Swagger 2.0 plus explicitly pinned common-type bundle | Subscription get/list |
| zixcel-cloudflare | Official OpenAPI JSON projected from large pinned source | Account get/list |
| zixcel-microsoft | Official partitioned Graph OpenAPI YAML | Selected user read |
| zixcel-apple | Official App Store Connect OpenAPI ZIP/member, hashes distinct | App get/list |
| zixcel-github | Official large OpenAPI JSON | Authenticated user / selected repository |
| zixcel-domain | Existing ValueDomain v2/DNS library and transport reused | Inventory, DNS export/inspection; native 0.3 adds explicit v2 record/migration writes |
| zixcel-aws | Existing SigV4/custody contracts and coordinator socket client | Not a bearer-native adapter; coordinator / authority composition required |

These are explicit initial read scopes, not claims that every vendor API is
supported. TypeScript writes, mail/finance decoders, DNS plans and signing retain
their existing owner boundaries. In particular usage/cost observations are not
issued invoices, a successful read is not an IAM grant inventory, and a visible
resource list is not proof of global completeness.

Owner repositories pin immutable common-library source revisions and retain
their own Cargo lock, tests, CI and daily dependency updates. They embed minimal
interface facts and original policies rather than copying entire SDKs or vendor
prose/schema bodies. Upstream source/version/hash, auxiliary document hashes,
selected IDs and compiler revision are recorded locally in `api/`.

To update a contract: inspect the official source/license; pin raw bytes and any
explicit document bundle; project; review the full catalog diff; export route
facts; update the owner policy pin; test exact request scope, response identity,
error/credential boundaries and the native process; review/merge through the
owner PR. No upstream addition automatically changes callable operations.

Native tag protection, artifact/provenance/checksum acceptance and publication
are separate from source completion. No tag, release, credentials, IAM, DNS or
real account operation is authorized or executed by the source tests. The common
`zix-extension/v1` contract remains read-only. Explicit operator writes use the
separate v2 manifest admission and terminal plan approval; providers still own
authority, one-use journals, vendor preconditions and postconditions. AWS signing
requires its existing custody authority, not an HTTP-method guess or passthrough.

Extensions using an OS Secret Service may declare
`runtime_environment: ["DBUS_SESSION_BUS_ADDRESS"]` in their reviewed manifest.
The dispatcher passes only an explicitly selected local Unix bus address; an
undeclared bus, remote transport, missing bus or arbitrary variable name is not
accepted. This grants access to that user's session services, so review the
extension's provenance before installation. It is not permission to unlock or
export credentials. Existing manifests inherit no session bus.

## Authentication and operation ownership

Login, refresh and profile enrollment belong to the selected provider and its
existing Crowsi custody contract. Installing an extension, discovering a manifest
or listing a capability does not log in or grant account permissions. The common
CLI does not introduce a global OAuth service or a second credential database.
A provider profile argument is an opaque local reference, never a password or
access token. Providers verify the reference's account/origin/workspace binding;
its display name is not authority. Long-lived values must stay in OS custody.

For an approved provider that uses Secret Service, the manifest must explicitly
select the local session bus. The dispatcher neither unlocks collections nor
copies broker-resolved secrets into its arguments/protocol. Legacy explicitly
selected `ZIX_*` credential environments remain supported, isolated from ambient
variables; only the declared credentials enter that child. Known credential bytes
and JSON-escaped equivalents are refused in responses. Providers remain responsible
for redacting secrets they resolve internally and never returning them.

Read and mutation effects are declared, not inferred from names or HTTP verbs.
V1 cannot admit operator-write. V2 requires plan-file, approval-sha256 and a
journal-directory in the provider protocol. Zix 0.5.0 obtains y/yes confirmation
or an invocation-local -y/--yes, and computes approval-sha256 when omitted by the
caller. The confirmation flags are not forwarded to providers. The digest binds
the registered executable, catalog, version, operation and payload; it is not an
identity signature, permission grant or replay journal. The provider owns expiry,
one-use consumption, durable outcomes and drift/readback. Unlocking is not approval.

Cancellation/timeout/output/protocol failures terminate the owned process tree;
the dispatcher never resubmits a request. Once an operator-write child has started,
a missing valid response is `operator_write_outcome_unknown`, with a static cause
and `provider_readback_required` diagnostic. Killing a process does not undo an
external mutation. Inspect the provider's durable receipt and target state; never
blindly approve and resend the same operation. Errors before launch retain their
ordinary admission/credential/approval diagnostics. An authenticated provider's
valid error/receipt is interpreted according to its own contract.

Domain operator entry is `zix domain …`; provider-specific aliases such as
`zixcel value-domain` are not separate namespaces. Domain native 0.3 uses the
existing v2 LocalPlan/terminal approval contract and provider-owned private
profiles/journals. Distribution and live acceptance stay with the Domain owner.
