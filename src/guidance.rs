//! Resolve descriptive actions against installed registrations; never run them.
use crate::{
    Error, Result,
    registry::{Entry, Registry},
};
use serde_json::{Value, json};
use zix_operation_contracts::guidance::{Action, Diagnostic, Guide, Trigger};
mod brief;
mod builtin;
mod discovery;
mod fetch;
mod legacy;
mod preflight;
pub(crate) use discovery::describe;
pub(crate) use preflight::context as preflight;
pub fn resolve(registry: &Registry, action: &Action) -> Result<Value> {
    if let Some(value) = builtin::resolve(action) {
        return Ok(value);
    }
    let entry = match registry.registered(&action.namespace) {
        Ok(v) => v,
        Err(Error("not_installed")) => {
            return Ok(
                json!({"purpose":action.purpose,"namespace":action.namespace,"operation":action.operation,"availability":"not_installed","runnable":false}),
            );
        }
        Err(e) => return Err(e),
    };
    let manifest = entry.manifest()?;
    let Some(command) = manifest.commands.iter().find(|c| c.id == action.operation) else {
        return Ok(
            json!({"purpose":action.purpose,"namespace":action.namespace,"operation":action.operation,"availability":"unsupported_operation","runnable":false}),
        );
    };
    if action.arguments.iter().any(|(k, v)| {
        !command
            .arguments
            .iter()
            .any(|a| a.name == *k && v.len() <= a.maximum)
    }) {
        return Ok(
            json!({"purpose":action.purpose,"availability":"incompatible_arguments","runnable":false}),
        );
    }
    let missing = command
        .arguments
        .iter()
        .filter(|a| {
            a.required && a.name != "approval-sha256" && !action.arguments.contains_key(&a.name)
        })
        .map(|a| a.name.clone())
        .collect::<Vec<_>>();
    let mut argv = vec!["zix".to_owned(), manifest.namespace.clone()];
    argv.extend(command.words.clone());
    for (k, v) in &action.arguments {
        argv.extend([format!("--{k}"), v.clone()]);
    }
    let credentials = command
        .environment
        .as_ref()
        .map_or(&manifest.credential_environment, |e| {
            &e.credential_environment
        });
    Ok(
        json!({"credential_environment":credentials,"integrity":"checked_on_execution","purpose":action.purpose,"namespace":manifest.namespace,"operation":command.id,"availability":if entry.enabled {"installed"} else {"disabled"},"effect":command.effect,"argv":argv,"missing_arguments":missing,"required_context":action.requires_context,"context_state":if action.requires_context.is_empty(){"not_required"}else{"unverified"},"runnable":entry.enabled && missing.is_empty() && action.requires_context.is_empty(),"authorization_state":"not_inferred","manifest_sha256":entry.manifest_sha256,"executable_sha256":entry.executable_sha256}),
    )
}
pub fn actions(registry: &Registry, guide: &Guide, reason: Option<Trigger>) -> Result<Vec<Value>> {
    guide
        .next_actions
        .iter()
        .filter(|a| reason.is_none_or(|r| a.on.contains(&r)))
        .map(|a| resolve(registry, a))
        .collect()
}
pub fn enrich(registry: &Registry, entry: &Entry, response: &mut Value) -> Result<()> {
    if response.get("diagnostic").is_none() {
        legacy::emit(entry, response)?;
        return Ok(());
    }
    let d: Diagnostic = serde_json::from_value(response["diagnostic"].clone())
        .map_err(|_| Error("invalid_diagnostic"))?;
    response["diagnostic"]["resolved_actions"] =
        json!(actions(registry, &d.guide, Some(d.reason))?);
    response["diagnostic"]["provenance"] = json!({"manifest_sha256":entry.manifest_sha256,"executable_sha256":entry.executable_sha256,"authority":"none"});
    response["diagnostic"]["inspect_argv"] =
        json!(brief::inspect(&entry.manifest()?.namespace, &d.guide));
    Ok(())
}
pub fn local_failure(code: &str) -> Value {
    let (reason, summary) = if code == "operator_write_outcome_unknown" {
        (
            "OutcomeUnknown",
            "OUTCOME UNKNOWN; NEXT provider readback. DO NOT replay.",
        )
    } else if ["not_installed", "disabled"].contains(&code) {
        (
            "MissingContext",
            "TOOL UNAVAILABLE; NEXT inspect installed extensions.",
        )
    } else if code.contains("credential") {
        (
            "MissingContext",
            "CREDENTIAL UNRESOLVED; NEXT inspect credential setup.",
        )
    } else {
        (
            "NeedsResolution",
            "OPERATION BLOCKED; NEXT inspect command requirements.",
        )
    };
    let argv = if ["not_installed", "disabled"].contains(&code) {
        vec!["zix", "extensions", "list", "--format", "json"]
    } else if code.contains("credential") {
        vec!["zix", "credential", "--help"]
    } else {
        vec!["zix", "capabilities", "--format", "json"]
    };
    json!({"error":code,"diagnostic":{"schema":"zix-local-diagnostic/v1","state":"NeedsResolution","reason":reason,"summary":summary,"retry":"no_automatic_retry","next_inspection_argv":argv,"authorization_state":"not_inferred","authority":"none"}})
}
