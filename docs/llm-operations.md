# Small-context LLM operation sequence

Use CLI 0.7.0 or newer. Any model that can select an operation and fill declared
string arguments can consume the same interface; model output grants no authority.

```sh
zix capabilities --compact --format json
zix describe microsoft --operation admin.organization --argument resource=security_defaults --compact --format json
```

The index contains names, effects and short descriptions, without argument trees.
The brief contains one operation, its command words, arguments, selected resource,
selector choices, missing inputs and typed requirements. It removes duplicated
catalog descriptions and per-action digests; `source` retains the selected
registration identity. `inspect_argv` retrieves the detailed view when needed.
Discovery invokes no provider HTTP endpoint or credential store.

1. Select one operation ID from the compact index; never invent an ID or flag.
2. Read its brief. If `missing_arguments` is nonempty, obtain the named inputs.
   Use `selector_choices` to select a resource, then inspect that exact selection.
3. Establish credential audience, target and user authorization. Declared rights
   do not prove the token has them; alternatives are choices, not cumulative grants.
4. Execute the declared command once. Keep partial `result` data when exit is 2.
5. `context` needs input; `authentication` needs issuer/expiry/identity inspection;
   `authorization` needs scope/consent/role/target evidence. Do not retry unchanged
   requests or grant broader rights based on a 403.
6. `outcome_unknown` needs readback. Never repeat its write automatically.

Only actions with the relevant effect, required context and existing authorization
may be selected. `runnable` describes shape and context, not live access or approval.
The root dispatcher checks selector-specific inputs before operator confirmation,
credential-store resolution and provider execution. The provider checks again.

Guided SDK 0.5.0 requires an owner-authored base guide for every operation. Generic
manifest-derived defaults no longer count as adoption. Legacy protocol v1 remains
readable for independently installed older packages, with its limited coverage
explicitly identified; compatibility is not a full-rollout claim.
