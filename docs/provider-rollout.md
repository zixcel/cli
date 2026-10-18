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
| zixcel-domain | Existing ValueDomain v2 library and transport reused | Domain inventory / renewal filter |
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
protocol remains read-only; AWS signing and native writes require their own
reviewed authority/execution design, not an HTTP-method guess or CLI passthrough.
