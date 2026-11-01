//! Openapi HTTP contracts with isolated state.

use mori::http;
use serde_json::Value;

#[test]
/// Reject contract drift that duplicates operation identifiers or omits mutation headers.
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "OpenAPI path keys intentionally contain literal parameter placeholders."
)]
fn generated_contract_has_unique_operation_ids() {
    let spec = serde_json::to_value(http::openapi()).unwrap();
    assert_eq!(spec["info"]["title"], "Mori API");
    assert!(spec["info"]["contact"].is_null());
    assert!(spec["info"]["license"].is_null());
    assert!(
        spec["components"]["schemas"]["Video"]["properties"]
            .get("embed_url")
            .is_none()
    );
    assert!(
        spec["components"]["schemas"]["Video"]["properties"]
            .get("watch_url")
            .is_none()
    );
    let paths = spec["paths"].as_object().unwrap();
    let mut ids = std::collections::HashSet::new();
    for (path, item) in paths {
        assert!(path.starts_with("/v1/"));
        for (method, operation) in item.as_object().unwrap() {
            if let Some(id) = operation.get("operationId").and_then(Value::as_str) {
                assert!(ids.insert(id.to_owned()), "duplicate operationId {id}");
                if ["post", "patch", "put", "delete"].contains(&method.as_str()) {
                    let policy = &operation["security"][0];
                    assert!(policy.get("origin").is_some());
                    if path != "/v1/auth/login" {
                        assert!(policy.get("session").is_some());
                        assert!(policy.get("csrf").is_some());
                    }
                }
            }
        }
    }
    assert!(paths["/v1/admin/videos/{id}/content"].get("head").is_some());
}
