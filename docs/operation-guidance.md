# Operation meaning, requirements and next actions

Use a short sequence: **MEANING → REQUIREMENTS → EVIDENCE → NEXT**. A meaning
description never grants execution authority. This follows sem-lang's separation
of meaning, missing context and external execution; it does not claim semantic
definition adoption or make its compiler a CLI prerequisite.

## LLM operation sequence

1. Read `zix capabilities --format json` and `zix prompt NAMESPACE`.
2. Before a remote request, inspect the selected operation and its public selectors:
   `zix describe NAMESPACE --operation OPERATION_ID --argument NAME=VALUE --format json`.
   Repeat `--argument` for another selector. Omit private inputs from discovery.
3. Supply the required target, profile and separately enrolled credentials. Use
   the selected registered command's declared arguments, not guessed shell text.
4. Inspect both the result and diagnostic. `Observed` describes an observation,
   not completion of the user's whole goal. `NeedsResolution` requires a decision.
5. Read `resolved_actions`: installed/disabled/missing operation, effect, argv,
   missing arguments, required context and exact package hashes. `runnable`
   requires the declared argument shape and no unresolved context condition;
   it never proves credentials, live permission, freshness or approval.
   `authorization_state=not_inferred` concerns this metadata only: reuse existing
   user authorization for the same scope instead of asking for it again.
6. Follow a relevant authorized step once, then verify its result. Use `required_context` to establish provider/authority applicability before a
   cross-tool handoff. Do not repeat
   an unchanged denied request or request broader permissions speculatively.

`zix describe NAMESPACE --guidance --format json` returns the full operation
catalog. Discovery uses the verified sealed native executable with an empty
credential environment and the SDK metadata path, with a five-second / 64-KiB
budget. It calls neither the provider handler nor its authentication flow.

## Concise diagnostic vocabulary

| Keywords | Meaning | Next decision |
|---|---|---|
| CONTEXT MISSING | Input/credential context is absent | Supply the named context |
| AUTH REJECTED | Authentication rejected | Check audience, expiry and identity |
| ACCESS DENIED | Access rejected; cause unconfirmed | Check scope, consent, role and target |
| EVIDENCE INCOMPLETE | Required observations are unavailable | Inspect the missing section |
| OUTCOME UNKNOWN | External effect unresolved | Read back before a separate new plan |
| OBSERVED | Operation returned observed evidence | Assess the user's next prerequisite |

`requirements_state=declared_not_verified` distinguishes necessary conditions
from confirmed missing permissions. Each requirement names its subject (for example,
`graph_credential`, `exchange_credential` or `operator`). `value` and optional
`alternatives` describe acceptable choices, not a request to grant all choices. An HTTP 403 alone does not establish which
scope/role/consent is missing. Credential storage does not grant provider access.
Keep Microsoft Graph and Exchange credential audiences distinct. Establish the
actual authoritative DNS before choosing a DNS adapter; registrar access alone
does not establish DNS authority. Keep Cloudflare application protection and
API credential/zone permission failures distinguishable and unresolved when
their causes are unavailable.

## Shared contract and ownership

- `zix-operation-contracts::guidance` owns bounded descriptive types and fixed
  observation classification, not a new semantic language or authorization engine.
- Providers own finite `Guide` entries: operation, selectors, intent, keywords,
  typed requirements, selector-specific required arguments, related operation IDs
  and documentation. The same entries
  drive `--guidance` and runtime diagnostics. Avoid separate hand-maintained copies.
- The native SDK validates guide-to-command binding before reading credentials.
  Selector-specific missing arguments stop before the provider handler, credentials
  or HTTP. The most specific matching selector wins; ambiguous equal-specificity rules
  are rejected. An unobserved property stays unobserved.
- The dispatcher validates responses and resolves related actions against its
  registered manifests. No PATH search, automatic install, inferred authority,
  arbitrary shell evaluation or automatic follow-up request is introduced.

Actions contain static public arguments only. Actual tenant IDs, domains, email
addresses, local paths, tokens and operational receipts stay outside public
catalogs and fixtures. Never echo provider messages/headers into guidance.
Root-level errors provide bounded local diagnostics and inspection argv.

## Protocol and compatibility

Guided providers declare `protocol=zix-provider/v2` in their existing manifest.
Requests and responses use that schema; responses add a closed
`diagnostic: zix-diagnostic/v1`. Existing `result`/`error` fields are preserved.
The dispatcher resolves actions after validation and records the source manifest
and executable hashes. Package archives retain the exact exported catalog and
its hash in `RELEASE.json`; installing it needs no additional registration flags.

Legacy v1 providers remain usable. Failed/incomplete legacy observations receive
a stderr diagnostic derived from their existing manifest, explicitly marked
`legacy_manifest_only`; no provider-specific scopes or handoffs are invented.
Guided `NeedsResolution` results exit 2 even when partial evidence is retained in
`result`. Existing protocol-v1 exit behavior remains unchanged. Parsers must
preserve partial evidence and inspect the diagnostic, rather than equate a
nonzero exit with absence of a provider response.

Upgrade the dispatcher before registering v2 providers. Old dispatchers reject
the new protocol; rollback requires compatible provider registrations. Do not
change an old plan's bindings or replay a journal when updating packages.

## Rollout and acceptance

| Owner | Coverage |
|---|---|
| Zix CLI / Credential | Discovery, common diagnostics, installed action resolution and secure custody guidance |
| Domain | Registrar vs authoritative DNS, provider scope, DNS planning and unresolved write readback |
| Cloudflare | Account/zone inventory, scoped API access and Domain DNS handoff |
| Microsoft | Graph/Exchange audiences, operation-specific scopes, consent/RBAC and DNS handoff |
| Other installed native providers | Common legacy diagnostics; specific semantics remain explicitly unavailable until owner adoption |

Acceptance uses synthetic state: missing context causes no remote call; 401/403
do not cause speculative retries/permission grants; unavailable tools are not
suggested as executable; selector drift rejects; partial observations do not
become completion; unresolved writes retain readback/no-replay instructions.
Run the actual dispatcher and independently packaged providers as consumers.
LLM use of discovery is separately recorded from deterministic fixture tests;
neither establishes live provider permissions or a general model success rate.
