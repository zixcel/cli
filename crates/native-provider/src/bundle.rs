//! Build-time only; no registry publishing, tag creation or credential use.
use std::{path::Path, process::ExitCode};
pub fn main(package: &str, version: &str, repository: &str) -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = (|| {
        if args.len() != 3 {
            return Err("usage_binary_output_target");
        }
        let source = std::env::var("ZIX_RELEASE_SOURCE").map_err(|_| "source_missing")?;
        zix_cli::release_bundle::native::bundle(
            zix_cli::release_bundle::native::Identity {
                package,
                version,
                repository,
                source: &source,
            },
            Path::new(&args[0]),
            Path::new(&args[1]),
            &args[2],
        )
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(2)
        }
    }
}
