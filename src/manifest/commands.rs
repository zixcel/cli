use super::{Manifest, clean, token};
use crate::{Error, Result};
use std::collections::HashSet;
pub(super) fn validate(manifest: &Manifest) -> Result<()> {
    let mut ids = HashSet::new();
    let mut commands = HashSet::new();
    for c in &manifest.commands {
        let mut args = HashSet::new();
        if !clean(&c.id, 100)
            || !ids.insert(&c.id)
            || !(c.effect == "read"
                || manifest.schema == "zix-extension/v2" && c.effect == "operator-write")
            || !clean(&c.description, 160)
            || c.words.is_empty()
            || c.words.len() > 6
            || c.words.iter().any(|w| !token(w))
            || !commands.insert(&c.words)
            || c.arguments.len() > 16
            || c.required_permissions.len() > 16
            || c.required_permissions.iter().any(|p| !clean(p, 128))
        {
            return Err(Error("invalid_manifest"));
        }
        if c.effect == "operator-write" && c.arguments.iter().any(|a| a.name == "yes") {
            return Err(Error("invalid_manifest"));
        }
        if c.effect == "operator-write"
            && !["plan-file", "approval-sha256", "journal-directory"]
                .iter()
                .all(|name| c.arguments.iter().any(|a| a.name == *name && a.required))
        {
            return Err(Error("invalid_operator_write"));
        }
        if let Some(scope) = &c.environment {
            for (selected, allowed) in [
                (
                    &scope.credential_environment,
                    &manifest.credential_environment,
                ),
                (&scope.runtime_environment, &manifest.runtime_environment),
            ] {
                let unique = selected.iter().collect::<HashSet<_>>();
                if unique.len() != selected.len()
                    || selected.iter().any(|name| !allowed.contains(name))
                {
                    return Err(Error("invalid_command_environment"));
                }
            }
        }
        for a in &c.arguments {
            if !token(&a.name) || !args.insert(&a.name) || !(1..=2048).contains(&a.maximum) {
                return Err(Error("invalid_manifest"));
            }
        }
    }
    for (i, a) in manifest.commands.iter().enumerate() {
        if manifest
            .commands
            .iter()
            .skip(i + 1)
            .any(|b| a.words.starts_with(&b.words) || b.words.starts_with(&a.words))
        {
            return Err(Error("ambiguous_operation"));
        }
    }
    Ok(())
}
