use std::collections::HashMap;

use crate::error::DaemonError;

/// # Errors
/// Returns an error if the generated hashmap can't be converted into a JSON
pub fn tuples_to_json(tuples: Vec<(String, Vec<(String, String)>)>) -> Result<String, DaemonError> {
    // Convert tuples nested hashmap
    let mut json_map: HashMap<String, HashMap<String, String>> = HashMap::new();

    let tuples_locked = tuples;
    for (group, pairs) in tuples_locked {
        let inner_map = pairs.into_iter().collect::<HashMap<_, _>>();

        json_map.insert(group, inner_map);
    }

    Ok(serde_json::to_string(&json_map)?)
}
