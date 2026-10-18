use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
mod archive;
mod licenses;
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(2)
        }
    }
}
fn run() -> Result<(), &'static str> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: package-native BINARY OUTPUT TARGET");
    }
    let target = &args[2];
    if !["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnu"].contains(&target.as_str()) {
        return Err("target_rejected");
    }
    let source = std::env::var("ZIX_RELEASE_SOURCE").map_err(|_| "source_missing")?;
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("source_rejected");
    }
    let binary = Path::new(&args[0]);
    let filename = binary
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("binary_rejected")?;
    if !["zix", "zix-google"].contains(&filename) {
        return Err("binary_rejected");
    }
    let name = format!(
        "{}-v{}-{target}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    let output = PathBuf::from(&args[1]);
    fs::create_dir_all(&output).map_err(|_| "output_unavailable")?;
    let directory = output.join(&name);
    fs::create_dir(&directory).map_err(|_| "output_already_exists")?;
    fs::copy(binary, directory.join(filename)).map_err(|_| "binary_copy_failed")?;
    for path in ["LICENSE", "NOTICE"] {
        fs::copy(path, directory.join(path)).map_err(|_| "owner_notice_missing")?;
    }
    if filename == "zix-google" {
        let result = Command::new(binary)
            .env_clear()
            .arg("--manifest")
            .output()
            .map_err(|_| "manifest_failed")?;
        if !result.status.success() || result.stdout.len() > 16384 {
            return Err("manifest_failed");
        }
        let _: serde_json::Value =
            serde_json::from_slice(&result.stdout).map_err(|_| "manifest_failed")?;
        fs::write(directory.join("google.extension.json"), result.stdout)
            .map_err(|_| "output_unavailable")?;
    }
    licenses::collect(&directory)?;
    let record = serde_json::json!({"schema":"zix-native-release/v1","package":env!("CARGO_PKG_NAME"),
        "version":env!("CARGO_PKG_VERSION"),"repository":env!("CARGO_PKG_REPOSITORY"),"source":source,"target":target,
        "binary":filename,"binary_sha256":archive::hash(&fs::read(binary).map_err(|_|"binary_unavailable")?)});
    fs::write(
        directory.join("RELEASE.json"),
        serde_json::to_vec_pretty(&record).map_err(|_| "invalid_record")?,
    )
    .map_err(|_| "output_unavailable")?;
    archive::finish(&output, &directory, &name)?;
    Ok(())
}
