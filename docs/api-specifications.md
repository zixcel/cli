# API specifications and reviewed commands

Zix is the common CLI entry. Its `api_spec` Rust library imports selected API
contracts; `api_command` connects reviewed aliases to installed providers.
Google-specific identity checks, billing semantics and operation completion live
in `zixcel-google`. Crowsi retains bounded process execution. API descriptions do
not become authorization policies or executable code.

## Owner workflow

1. Download an official specification separately. Inspect its source and license;
   retain upstream notices with any redistributed specification. Never include
   account data or credentials in a specification or review file.
2. Pin the exact source SHA256 in a `zix-api-review/v1` file and select operation
   IDs. Import only selected operations and their local reference closure.
3. Review method, path, parameters, scopes, request/response schemas and global
   context in the resulting catalog. A null binding makes an operation visible
   but not executable. No HTTP verb is interpreted as an effect or permission.
4. An owner may add a binding with `namespace`, `command_id` and the exact
   installed `manifest_sha256`. This deliberately maps an API operation to a
   reviewed provider command; its flags are the provider's declared flags.
5. Commit and review the catalog. Supply its exact SHA256 at execution. Changing
   catalog bytes or provider manifests requires explicit re-review. Executables
   remain independently hash-bound in the existing extension registry.

```sh
zix api import /absolute/service.json /absolute/review.json > /private/catalog.json
zix api describe /private/catalog.json
zix api run /private/catalog.json REVIEWED_CATALOG_SHA256 operation.id --flag value
```

`run` uses the provider's native protocol, validation, credential selection and
Crowsi execution limits. It never sends a request to the catalog's server URL.
The current protocol accepts read-only providers and string arguments. The
catalog preserves API schemas for review and drift detection; it is not a full
JSON Schema validator, SDK generator, or a guarantee that a provider implements
all source constraints. Provider owners must test that correspondence. A binding
is not a claim that an upstream OAuth scope is an effective IAM grant.

## Updating independently

Download the candidate spec, explicitly update its source hash, and import to a
second file. Review the complete report and contracts:

```sh
zix api diff /private/catalog.json /private/candidate.json
```

Exit 0 means exact catalog equality; exit 2 means differences or an error (JSON
distinguishes the two). The report includes added, removed and changed selected
IDs plus source metadata/hash changes. It does not declare changes compatible.
Referenced schema constraints, scopes, server/security definitions and bindings
participate in comparisons. Unselected upstream additions change the source
hash but are never enabled. Even documentation-only source changes require
review of the new source hash. There is no startup discovery, polling, scheduled
update, permission expansion or automatic adoption.

## Supported input and limits

- JSON OpenAPI 3.0.x / 3.1.x and Google Discovery v1. YAML and newer dialects
  fail closed. This is a selected-contract importer, not full spec conformance.
- OpenAPI operation IDs must be explicit and unique. Path-item `$ref` is
  unsupported and rejected. Parameter/schema/response local references are
  preserved; external/file/network references are never fetched.
- Discovery named schema references and nested resources are supported.
  Recursive local schemas are retained without infinite expansion.
- Inputs/catalogs: 8 MiB; review: 64 KiB; selected operations: 32; references per
  operation: 128; JSON depth: 64; nodes: 200,000. Files must satisfy the existing
  absolute-path, ownership, mode and no-symlink checks. Duplicate keys fail.
- Exact numeric lexemes are retained, including large integers and decimals.
  Output is bounded, deterministic compact JSON. Descriptions remain untrusted JSON data.

Mutation approval, IAM discovery, destructive-operation postconditions and
unknown-outcome handling belong to reviewed owner policies. They are not
implemented by adding a spec binding. This change does not publish a binary or
execute real Cloud operations.

See the [offline playground](../playground/api-catalog/README.md). Specification
references: [OpenAPI](https://spec.openapis.org/oas/v3.1.2.html) and
[Google Discovery](https://developers.google.com/discovery/v1/reference/apis).
