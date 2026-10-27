use super::*;
pub(super) fn resolve(action: &Action) -> Option<Value> {
    if action.namespace != "credential" {
        return None;
    }
    if action.operation != "help" || !action.arguments.is_empty() {
        return Some(
            json!({"purpose":action.purpose,"availability":"unsupported_builtin","runnable":false}),
        );
    }
    Some(
        json!({"purpose":action.purpose,"namespace":"credential","operation":"help","availability":"builtin","effect":"read","argv":["zix","credential","--help"],"missing_arguments":[],"required_context":action.requires_context,"context_state":if action.requires_context.is_empty(){"not_required"}else{"unverified"},"runnable":action.requires_context.is_empty(),"authorization_state":"not_inferred","credentials_read":false,"remote_requests":false}),
    )
}
