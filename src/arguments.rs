use crate::{
    Error, Result,
    manifest::{Command, Manifest},
};
use std::collections::BTreeMap;
pub struct Selected<'a> {
    pub(crate) command: &'a Command,
    pub(crate) arguments: BTreeMap<String, String>,
    pub(crate) assume_yes: bool,
}
pub fn select<'a>(manifest: &'a Manifest, args: &[String]) -> Result<Selected<'a>> {
    if args.len() > 40
        || args
            .iter()
            .any(|v| v.len() > 2048 || v.bytes().any(|b| b.is_ascii_control()))
    {
        return Err(Error("invalid_arguments"));
    }
    let mut matches = manifest
        .commands
        .iter()
        .filter(|c| args.starts_with(&c.words));
    let command = matches.next().ok_or(Error("unknown_operation"))?;
    if matches.next().is_some() {
        return Err(Error("ambiguous_operation"));
    }
    let (remaining, assume_yes) = crate::confirmation::split(&args[command.words.len()..]);
    if assume_yes && command.effect != "operator-write" {
        return Err(Error("confirmation_not_supported"));
    }
    if !remaining.len().is_multiple_of(2) {
        return Err(Error("invalid_arguments"));
    }
    let mut arguments = BTreeMap::new();
    for pair in remaining.chunks_exact(2) {
        let name = pair[0]
            .strip_prefix("--")
            .ok_or(Error("invalid_arguments"))?;
        let spec = command
            .arguments
            .iter()
            .find(|a| a.name == name)
            .ok_or(Error("unknown_argument"))?;
        if pair[1].is_empty()
            || pair[1].len() > spec.maximum
            || arguments.insert(name.to_owned(), pair[1].clone()).is_some()
        {
            return Err(Error("invalid_arguments"));
        }
    }
    if command.arguments.iter().any(|a| {
        a.required
            && !arguments.contains_key(&a.name)
            && !(command.effect == "operator-write" && a.name == "approval-sha256")
    }) {
        return Err(Error("missing_argument"));
    }
    Ok(Selected {
        command,
        arguments,
        assume_yes,
    })
}
