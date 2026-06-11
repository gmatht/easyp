use std::collections::HashMap;
use futures::channel::mpsc::UnboundedReceiver;

pub fn handle_stream(
    admin_keys: &HashMap<String, String>,
) -> Result<UnboundedReceiver<String>, String> {
    let _expected_key = admin_keys.get("logs")
        .ok_or("Logs admin key not found".to_string())?;
    Ok(crate::logs_admin::subscribe_to_log_stream())
}
