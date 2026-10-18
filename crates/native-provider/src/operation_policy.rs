use crate::{Error, Operation, Result, validation};
use std::collections::BTreeSet;
use zix_cli::api_spec::Routes;
pub(crate) fn validate(routes: &Routes, op: &Operation) -> Result<()> {
    let route = routes
        .operations
        .iter()
        .find(|r| r.id == op.api)
        .ok_or(Error("missing_provider_route"))?;
    let names = op
        .command
        .arguments
        .iter()
        .map(|a| &a.name)
        .collect::<BTreeSet<_>>();
    if route.method != "GET"
        || !safe_path(&route.path)
        || !route.path.starts_with('/')
        || op.command.effect != "read"
        || op.command.arguments.iter().any(|a| !a.required)
        || op.path_arguments.len() > 16
        || op.query.len() > 8
        || op.headers.len() > 4
        || op.argument_rules.keys().collect::<BTreeSet<_>>() != names
        || op.identity.len() > 8
        || op.projection.fields.is_empty()
        || op.projection.fields.len() > 16
        || op
            .projection
            .required
            .iter()
            .any(|k| !op.projection.fields.contains_key(k))
        || !validation::pointer(&op.projection.pointer)
        || op
            .projection
            .success_pointer
            .as_ref()
            .is_some_and(|p| !validation::pointer(p))
        || op
            .projection
            .fields
            .iter()
            .any(|(k, p)| !zix_cli::manifest::token(k) || !validation::pointer(p))
    {
        return Err(Error("invalid_provider_operation"));
    }
    for (parameter, arg) in &op.path_arguments {
        if !route.path.contains(&format!("{{{parameter}}}")) || !names.contains(arg) {
            return Err(Error("unbound_scope_argument"));
        }
    }
    let placeholders = names
        .iter()
        .map(|n| ((*n).clone(), "x".to_owned()))
        .collect();
    for identity in &op.identity {
        validation::substitute(&identity.expected, &placeholders)?;
        if !validation::pointer(&identity.pointer) {
            return Err(Error("invalid_identity_check"));
        }
    }
    for name in names {
        if !op.path_arguments.values().any(|a| a == name)
            && !op
                .identity
                .iter()
                .any(|i| i.expected.contains(&format!("{{{name}}}")))
        {
            return Err(Error("unused_scope_argument"));
        }
    }
    Ok(())
}
pub(crate) fn safe_path(value: &str) -> bool {
    value.len() <= 2048
        && !value.contains(['?', '#', '%', '\\'])
        && !value.contains("//")
        && !value.split('/').any(|s| s == "." || s == "..")
        && value.is_ascii()
        && !value.bytes().any(|b| b.is_ascii_control() || b == b' ')
}
