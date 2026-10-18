use super::Catalog;
use serde_json::Value;
pub fn diff(old: &Catalog, new: &Catalog) -> Value {
    let before = old
        .operations
        .iter()
        .map(|o| (&o.id, o))
        .collect::<std::collections::BTreeMap<_, _>>();
    let after = new
        .operations
        .iter()
        .map(|o| (&o.id, o))
        .collect::<std::collections::BTreeMap<_, _>>();
    let added = after
        .keys()
        .filter(|k| !before.contains_key(*k))
        .collect::<Vec<_>>();
    let removed = before
        .keys()
        .filter(|k| !after.contains_key(*k))
        .collect::<Vec<_>>();
    let changed = after
        .iter()
        .filter(|(k, v)| before.get(*k).is_some_and(|old| old != *v))
        .map(|(k, _)| k)
        .collect::<Vec<_>>();
    serde_json::json!({"schema":"zix-api-diff/v1", "source_changed":old.source != new.source,
        "added":added,"removed":removed,"changed":changed,"equal":old == new,
        "automatic_adoption":false})
}
