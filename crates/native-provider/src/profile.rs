use crate::{
    Error, Policy, Result,
    operation_policy::{safe_path, validate},
};
use std::collections::BTreeSet;
use zix_cli::{
    api_spec::{Routes, json},
    files,
    manifest::Manifest,
};
pub struct Provider {
    pub(crate) policy: Policy,
    pub(crate) routes: Routes,
    pub(crate) manifest: Manifest,
}
impl Provider {
    pub fn load(routes: &[u8], policy: &[u8], version: &str) -> Result<Self> {
        let policy: Policy = serde_json::from_value(json::parse(policy, 65536)?)
            .map_err(|_| Error("invalid_provider_policy"))?;
        if policy.schema != "zix-native-policy/v1"
            || policy.version != version
            || files::digest(routes) != policy.routes_sha256
        {
            return Err(Error("provider_policy_integrity_mismatch"));
        }
        let routes: Routes = serde_json::from_value(json::parse(routes, 65536)?)
            .map_err(|_| Error("invalid_provider_routes"))?;
        let origin =
            url::Url::parse(&policy.origin).map_err(|_| Error("invalid_provider_origin"))?;
        if routes.schema != "zix-api-routes/v1"
            || !files::valid_digest(&routes.source.sha256)
            || origin.scheme() != "https"
            || origin.host_str().is_none()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || !safe_path(&policy.base_path)
            || (!policy.base_path.is_empty()
                && (!policy.base_path.starts_with('/') || policy.base_path.ends_with('/')))
            || routes.operations.len() > 32
        {
            return Err(Error("invalid_provider_policy"));
        }
        let manifest = Manifest {
            schema: "zix-extension/v1".into(),
            package: policy.package.clone(),
            version: version.into(),
            namespace: policy.namespace.clone(),
            platform: "linux-x86_64".into(),
            protocol: "zix-provider/v1".into(),
            credential_environment: vec![policy.credential_environment.clone()],
            commands: policy
                .operations
                .iter()
                .map(|o| o.command.clone())
                .collect(),
            timeout_ms: 20000,
        };
        manifest.validate()?;
        let mut ids = BTreeSet::new();
        for r in &routes.operations {
            if !ids.insert(&r.id) {
                return Err(Error("duplicate_provider_route"));
            }
        }
        for op in &policy.operations {
            validate(&routes, op)?;
            let route = routes
                .operations
                .iter()
                .find(|r| r.id == op.api)
                .ok_or(Error("missing_provider_route"))?;
            let base = format!("{}{}", policy.origin, policy.base_path);
            if !route
                .servers
                .iter()
                .any(|s| s.trim_end_matches('/') == base.trim_end_matches('/'))
            {
                return Err(Error("provider_origin_binding_mismatch"));
            }
        }
        Ok(Self {
            policy,
            routes,
            manifest,
        })
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}
