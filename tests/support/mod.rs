use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};
pub struct Context {
    pub root: tempfile::TempDir,
    pub executable: PathBuf,
    pub manifest: PathBuf,
}
impl Context {
    pub fn new(timeout: u64) -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let directory = root.path().join("provider with spaces");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let executable = directory.join("probe");
        fs::copy(env!("CARGO_BIN_EXE_zix-contract-probe"), &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let manifest = directory.join("extension.json");
        fs::write(&manifest,json!({"schema":"zix-extension/v1","package":"sample-provider","version":"0.1.0",
            "namespace":"sample","platform":"linux-x86_64","protocol":"zix-provider/v1","timeout_ms":timeout,
            "credential_environment":["ZIX_TEST_ACCESS_TOKEN"],
            "commands":[{"id":"sample.inspect","words":["inspect"],"effect":"read","description":"Synthetic provider inspection.",
                "required_permissions":[],"arguments":[{"name":"mode","required":true,"maximum":32},{"name":"value","required":false,"maximum":200}]}]}).to_string()).unwrap();
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o600)).unwrap();
        Self {
            root,
            executable,
            manifest,
        }
    }
    pub fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_zix"));
        c.env_clear().env("XDG_CONFIG_HOME", self.root.path());
        c
    }
    pub fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    pub fn install(&self) -> Output {
        let sha = |p: &Path| zix_cli::files::digest(&fs::read(p).unwrap());
        self.run(&[
            "extensions",
            "add",
            "--manifest",
            self.manifest.to_str().unwrap(),
            "--manifest-sha256",
            &sha(&self.manifest),
            "--executable",
            self.executable.to_str().unwrap(),
            "--executable-sha256",
            &sha(&self.executable),
        ])
    }
}
