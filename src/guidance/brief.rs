use super::*;
use crate::manifest::{Command, Manifest};
use std::collections::{BTreeMap, BTreeSet};
use zix_operation_contracts::guidance::Catalog;

pub(super) fn inspect(namespace: &str, guide: &Guide) -> Vec<String> {
    let mut argv = vec![
        "zix".into(),
        "describe".into(),
        namespace.into(),
        "--operation".into(),
        guide.operation.clone(),
    ];
    for (name, value) in &guide.selectors {
        argv.extend(["--argument".into(), format!("{name}={value}")]);
    }
    argv.extend(["--format".into(), "json".into()]);
    argv
}

pub(super) fn operation(
    registry: &Registry,
    entry: &Entry,
    manifest: &Manifest,
    command: &Command,
    catalog: &Catalog,
    guide: &Guide,
    arguments: &BTreeMap<String, String>,
) -> Result<Value> {
    let mut selected = guide.clone();
    selected.selectors = arguments
        .iter()
        .filter(|(k, _)| k.as_str() != "approval-sha256")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let required = command
        .arguments
        .iter()
        .filter(|a| a.required && a.name != "approval-sha256")
        .map(|a| a.name.clone())
        .chain(guide.required_arguments.clone())
        .collect::<BTreeSet<_>>();
    let mut choices: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for g in catalog.entries.iter().filter(|g| g.operation == command.id) {
        for (k, v) in &g.selectors {
            choices.entry(k).or_default().insert(v);
        }
    }
    let mut argv = vec!["zix".to_owned(), manifest.namespace.clone()];
    argv.extend(command.words.clone());
    let next = actions(registry, guide, None)?
        .into_iter()
        .map(|a| {
            let mut v = serde_json::Map::new();
            for key in [
                "namespace",
                "operation",
                "purpose",
                "availability",
                "effect",
                "argv",
                "missing_arguments",
                "required_context",
                "context_state",
                "runnable",
                "authorization_state",
            ] {
                if let Some(value) = a.get(key) {
                    v.insert(key.into(), value.clone());
                }
            }
            Value::Object(v)
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"schema":"zix-operation-brief/v1","namespace":manifest.namespace,"version":manifest.version,
        "operation":command.id,"intent":guide.intent,"keywords":guide.keywords,"effect":command.effect,
        "command":argv,"arguments":command.arguments.iter().filter(|a|a.name!="approval-sha256").collect::<Vec<_>>(),
        "selected_arguments":selected.selectors,"selector_choices":choices,"required_arguments":required,
        "missing_arguments":required.iter().filter(|k|!selected.selectors.contains_key(*k)).collect::<Vec<_>>(),
        "requirements":guide.requirements,"requirements_state":"declared_not_verified","next":next,
        "authority":"none","remote_requests":false,"credentials_read":false,"retry":"no_automatic_retry",
        "inspect_argv":inspect(&manifest.namespace,&selected),
        "source":{"manifest_sha256":entry.manifest_sha256,"executable_sha256":entry.executable_sha256}}),
    )
}
