use crate::{Error, Result, install};
use std::{collections::BTreeMap, path::Path};
pub fn run(path: &Path, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("add") => {
            if args.len() != 9 {
                return Err(Error("invalid_arguments"));
            }
            let mut flags = BTreeMap::new();
            for pair in args[1..].chunks_exact(2) {
                if ![
                    "--manifest",
                    "--manifest-sha256",
                    "--executable",
                    "--executable-sha256",
                ]
                .contains(&pair[0].as_str())
                    || flags.insert(pair[0].as_str(), pair[1].as_str()).is_some()
                {
                    return Err(Error("invalid_arguments"));
                }
            }
            install::mutate(path, |r| {
                install::add(
                    r,
                    Path::new(flags["--manifest"]),
                    flags["--manifest-sha256"],
                    Path::new(flags["--executable"]),
                    flags["--executable-sha256"],
                )
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
