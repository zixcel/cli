# Small-context LLM operation sequence

Use CLI 0.7.0 or newer. Any model that can select an operation and fill declared
string arguments can consume the same interface; model output grants no authority.

```sh
zix capabilities --compact --format json
zix capabilities --namespace microsoft --compact --format json
zix describe microsoft --operation admin.organization --argument resource=security_defaults --compact --format json
```

The first index contains provider names, operation counts and effects. Select a
namespace to obtain only its operation IDs and short descriptions, without argument trees.
The brief contains one operation, its command words, arguments, selected resource,
selector choices, missing inputs and typed requirements. It removes duplicated
catalog descriptions and per-action digests; `source` retains the selected
registration identity. `inspect_argv` retrieves the detailed view when needed.
Discovery invokes no provider HTTP endpoint or credential store.

1. Select a provider from the index, then read its compact namespace index. Select
   one operation ID; never invent an ID or flag.
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

## Small-model caller view

Small models can confuse routing when the entire JSON envelope is placed in the prompt. Keep the full metadata in the caller, render IDs and keywords, then put the request last. The optional standard-library reference formatter reads stdin only:

```sh
zix capabilities --compact --format json | python3 docs/llm_view.py
zix capabilities --namespace microsoft --compact --format json | python3 docs/llm_view.py
```

Give the model the returned `text` and constrain decoding to one literal ID from `choices`. For very small models, return that ID directly and let the caller construct the JSON envelope with `select_id(view, text, allowed_choices)`. Native Zix transport remains JSON; the model need not generate its framing. Keep task authorization, effects, target identifiers, hashes and full requirements outside the model's selection step. `select(view, answer, allowed_choices)` checks the registered ID against an allowlist supplied by the caller's authorized workflow. It never executes a command. An allowlist is not inferred from model output.

After selection, describe that operation, supply known inputs and check the returned requirements before a request. `gate(response)` blocks missing inputs, denied access, incomplete observations and unknown writes independently of the model. An `inspect_only` view grants no execution authority. Never use a model's choice to bypass a blocked gate or manufacture missing context. The execution adapter must use registered command words, validated argument names and an argument vector with `shell=False`; credentials remain in Zix/Crowsi.

Use a short system instruction such as “Match the final request to one listed choice. Use meaning and keywords. Return only the matching ID.” The following request layout avoids repeated catalogs:

```text
CHOICES AND MEANINGS:
<text from one view>
ALLOWED CHOICES: <registered IDs>
REQUEST: <authorized task, placed last>
```

A smaller prompt does not make every small model reliable. Qualification must record model revision, quantization, runtime, prompt view, task cases and failures. Routing accuracy and policy enforcement are separate checks; the model never decides whether to grant permissions, replay a write or treat partial inventory as absence.
