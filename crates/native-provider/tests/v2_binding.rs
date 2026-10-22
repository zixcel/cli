mod support;
use support::*;
use zix_native_provider::validate_request;
fn bound() -> (zix_native_provider::Manifest, zix_native_provider::Request) {
    let p = provider();
    let mut manifest = p.manifest().clone();
    manifest.schema = "zix-extension/v2".into();
    let bytes = std::fs::read("/proc/self/exe").unwrap();
    let binding = zix_operation_contracts::Binding::for_catalog(
        &manifest.package,
        &manifest.version,
        &serde_json::to_value(&manifest).unwrap(),
        &zix_cli::files::digest(&bytes),
    );
    let mut req = request();
    req.binding = Some(binding);
    (manifest, req)
}
#[test]
fn v2_dispatcher_binding_is_accepted_and_required() {
    let (m, mut req) = bound();
    validate_request(&m, &req).unwrap();
    req.binding = None;
    assert_eq!(
        validate_request(&m, &req).unwrap_err().0,
        "provider_binding_missing"
    );
}
#[test]
fn changed_binary_catalog_version_or_package_is_rejected() {
    for i in 0..4 {
        let (m, mut req) = bound();
        let b = req.binding.as_mut().unwrap();
        match i {
            0 => b.executable_sha256 = "a".repeat(64),
            1 => b.catalog_sha256 = "b".repeat(64),
            2 => b.version = "other".into(),
            _ => b.package = "other".into(),
        }
        assert_eq!(
            validate_request(&m, &req).unwrap_err().0,
            "provider_binding_mismatch"
        );
    }
}
#[test]
fn v1_profile_does_not_accept_a_v2_binding_as_extra_authority() {
    let (_, mut req) = bound();
    assert_eq!(
        validate_request(provider().manifest(), &req).unwrap_err().0,
        "unexpected_provider_binding"
    );
    req.binding = None;
    validate_request(provider().manifest(), &req).unwrap();
}
