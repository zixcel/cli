//! Owner-authored operation meaning. No provider request, credential read or execution grant.
use crate::{Error, Manifest, Result};
use zix_operation_contracts::guidance::{Catalog, Guide};
pub fn catalog(manifest: &Manifest, overrides: &[u8]) -> Result<Catalog> {
    // Pretty owner source may exceed its compact wire representation.
    let value = zix_cli::api_spec::json::parse(overrides, 131072)?;
    let entries: Vec<Guide> =
        serde_json::from_value(value).map_err(|_| Error("invalid_guidance"))?;
    let catalog = Catalog {
        schema: "zix-guidance-catalog/v1".into(),
        namespace: manifest.namespace.clone(),
        package: manifest.package.clone(),
        version: manifest.version.clone(),
        entries,
    };
    validate_catalog(manifest, &catalog)?;
    Ok(catalog)
}
/// Command descriptions are a bounded projection of the same authored meaning.
pub fn decorate(manifest: &mut Manifest, overrides: &[u8]) -> Result<()> {
    let catalog = catalog(manifest, overrides)?;
    for command in &mut manifest.commands {
        if let Some(g) = catalog
            .entries
            .iter()
            .find(|g| g.operation == command.id && g.selectors.is_empty())
        {
            let description = format!(
                "{}: {}",
                g.keywords
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
                g.intent
            );
            if description.len() > 160 {
                return Err(Error("guidance_description_limit"));
            }
            command.description = description;
        }
    }
    Ok(())
}
pub fn validate_catalog(manifest: &Manifest, catalog: &Catalog) -> Result<()> {
    if !catalog.validate()
        || catalog.namespace != manifest.namespace
        || catalog.package != manifest.package
        || catalog.version != manifest.version
    {
        return Err(Error("invalid_guidance"));
    }
    for guide in &catalog.entries {
        let c = manifest
            .commands
            .iter()
            .find(|c| c.id == guide.operation)
            .ok_or(Error("invalid_guidance_operation"))?;
        if guide
            .required_arguments
            .iter()
            .any(|k| !c.arguments.iter().any(|a| a.name == *k))
            || guide.selectors.iter().any(|(k, v)| {
                !c.arguments
                    .iter()
                    .any(|a| a.name == *k && v.len() <= a.maximum)
            })
        {
            return Err(Error("invalid_guidance_selector"));
        }
    }
    if manifest.commands.iter().any(|c| {
        !catalog
            .entries
            .iter()
            .any(|g| g.operation == c.id && g.selectors.is_empty())
    }) {
        return Err(Error("guidance_operation_missing"));
    }
    Ok(())
}

/// Lift a read-only route adapter into the guided host without changing its API policy.
pub fn from_provider(provider: &crate::Provider, overrides: &[u8]) -> Result<(Manifest, Catalog)> {
    let mut manifest = provider.manifest().clone();
    manifest.protocol = "zix-provider/v2".into();
    decorate(&mut manifest, overrides)?;
    let catalog = catalog(&manifest, overrides)?;
    Ok((manifest, catalog))
}

mod diagnostic;
pub use diagnostic::diagnose;
