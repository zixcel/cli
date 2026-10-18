# Import an API contract without accessing an account

The adjacent OpenAPI document is original synthetic example data. It uses the
reserved `.invalid` domain. It does not point to a real service. Run from the
repository root with the existing Rust environment:

```sh
cargo run --locked --bin zix -- api import \
  "$PWD/playground/api-catalog/service.openapi.json" \
  "$PWD/playground/api-catalog/review.json" > /tmp/zix-example-catalog.json
cargo run --locked --bin zix -- api describe /tmp/zix-example-catalog.json
cargo run --locked --bin zix -- api diff \
  /tmp/zix-example-catalog.json /tmp/zix-example-catalog.json
```

The catalog contains one operation and its referenced response schema. Its
binding is null: inspection succeeds, execution refuses `api_operation_not_bound`.
No registry, credentials, API calls, dynamic source fetching or Cloud changes
are needed. `review.json` pins these exact source bytes; editing the spec without
reviewing its hash stops import.

`tests/api_cli.rs` demonstrates the next step using the real Zix executable and a
synthetic installed native provider. It binds `sample.inspect` to the exact
provider manifest, executes it through Crowsi, and verifies argument/environment
handling. The probe executable is a development test helper, never a release
provider. Integrity tests exercise changed catalog/provider definitions, unknown
operations, unbound operations and symlink refusal.

For a real owner, implement resource/credential/response policies in its provider
and test correspondence with the selected contract. Register verified provider
bytes through the existing extension flow, add its explicit binding, and review
the resulting catalog hash. The importer does not implement an HTTP endpoint by
itself. [Architecture, bounds and update procedure](../../docs/api-specifications.md).
