use crate::{Error, Result, install};
use std::{collections::BTreeMap, path::Path};
pub fn run(path: &Path, args: &[String]) -> Result<()> {
    if args.is_empty() || args == ["--help"] {
        println!(
            "zix extensions list [--format json]\nzix extensions inspect NAMESPACE [--format json]\nzix extensions add --manifest FILE --manifest-sha256 HASH --executable FILE --executable-sha256 HASH\nzix extensions replace --manifest FILE --manifest-sha256 HASH --executable FILE --executable-sha256 HASH --expected-manifest-sha256 HASH --expected-executable-sha256 HASH\nzix extensions enable|disable|remove NAMESPACE\n\nInspection includes disabled registrations. Replace preserves enablement and requires current hashes; it never removes the old entry first. Removal unregisters only, leaving package files intact."
        );
        return Ok(());
    }
    match args.first().map(String::as_str) {
        Some("list" | "inspect") => return crate::extension_inventory::run(path, args),
        Some(operation @ ("add" | "replace")) => {
            let replacing = operation == "replace";
            if args.len() != if replacing { 13 } else { 9 } {
                return Err(Error("invalid_arguments"));
            }
            let allowed = [
                "--manifest",
                "--manifest-sha256",
                "--executable",
                "--executable-sha256",
                "--expected-manifest-sha256",
                "--expected-executable-sha256",
            ];
            let mut flags = BTreeMap::new();
            for pair in args[1..].chunks_exact(2) {
                if !allowed[..if replacing { 6 } else { 4 }].contains(&pair[0].as_str())
                    || flags.insert(pair[0].as_str(), pair[1].as_str()).is_some()
                {
                    return Err(Error("invalid_arguments"));
                }
            }
            if flags.len() != if replacing { 6 } else { 4 } {
                return Err(Error("invalid_arguments"));
            }
            install::mutate(path, |registry| {
                let (manifest, manifest_sha, executable, executable_sha) = (
                    Path::new(flags["--manifest"]),
                    flags["--manifest-sha256"],
                    Path::new(flags["--executable"]),
                    flags["--executable-sha256"],
                );
                if replacing {
                    install::replace(
                        registry,
                        manifest,
                        manifest_sha,
                        executable,
                        executable_sha,
                        flags["--expected-manifest-sha256"],
                        flags["--expected-executable-sha256"],
                    )
                } else {
                    install::add(registry, manifest, manifest_sha, executable, executable_sha)
                }
            })?;
        }
        Some(operation @ ("enable" | "disable" | "remove")) if args.len() == 2 => {
            install::mutate(path, |r| install::change(r, &args[1], operation))?;
        }
        _ => return Err(Error("invalid_arguments")),
    }
    println!("{{\"state\":\"updated\"}}");
    Ok(())
}
