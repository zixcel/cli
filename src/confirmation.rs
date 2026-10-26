//! Explicit invocation-local consent; never inherited through an environment variable.
use crate::{Error, Result};
use std::io::{BufRead, IsTerminal, Read, Write};
pub(crate) fn split(args: &[String]) -> (Vec<String>, bool) {
    let mut result = Vec::new();
    let mut yes = false;
    let mut i = 0;
    while i < args.len() {
        if matches!(args[i].as_str(), "-y" | "--yes") {
            yes = true;
            i += 1;
        } else {
            result.push(args[i].clone());
            i += 1;
            if i < args.len() {
                result.push(args[i].clone());
                i += 1;
            }
        }
    }
    (result, yes)
}
pub(crate) fn leading(args: &[String]) -> Vec<String> {
    let n = args
        .iter()
        .take_while(|a| matches!(a.as_str(), "-y" | "--yes"))
        .count();
    let mut result = args[n..].to_vec();
    if n > 0 {
        if result.first().is_some_and(|s| s == "credential") && result.len() > 1 {
            result.insert(2, "--yes".into());
        } else {
            result.push("--yes".into());
        }
    }
    result
}
pub(crate) fn accepted(line: &str) -> bool {
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
pub(crate) fn require_interactive(yes: bool) -> Result<()> {
    if !yes && !std::io::stdin().is_terminal() {
        return Err(Error("interactive_authorization_required"));
    }
    Ok(())
}
pub(crate) fn ask(yes: bool) -> Result<()> {
    require_interactive(yes)?;
    if yes {
        eprintln!("Confirmed by --yes.");
        return Ok(());
    }
    eprint!("Proceed? [y/N]: ");
    std::io::stderr()
        .flush()
        .map_err(|_| Error("approval_unavailable"))?;
    let mut input = String::new();
    std::io::stdin()
        .lock()
        .take(16)
        .read_line(&mut input)
        .map_err(|_| Error("approval_unavailable"))?;
    if !accepted(&input) {
        return Err(Error("approval_declined"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consent_is_explicit_and_values_are_not_flags() {
        for s in ["y\n", "yes\n", "Y", " YES "] {
            assert!(accepted(s));
        }
        for s in ["", "n", "no", "true", "yesterday", "a"] {
            assert!(!accepted(s));
        }
        let args = ["--value", "--yes", "-y"].map(String::from);
        assert_eq!(split(&args), (vec!["--value".into(), "--yes".into()], true));
        assert_eq!(split(&args[..2]), (args[..2].to_vec(), false));
    }
}
