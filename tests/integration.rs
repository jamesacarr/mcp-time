use mcp_time::server::{ConvertTimeParams, GetCurrentTimeParams, TimeServer};
use rmcp::{handler::server::wrapper::Parameters, model::RawContent};

/// Extract text content from the first element of a CallToolResult.
fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    match &result.content[0].raw {
        RawContent::Text(t) => t.text.clone(),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn server_exposes_exactly_two_tools_with_metadata() {
    let server = TimeServer::new();
    let tools = server.tool_router.list_all();

    assert_eq!(
        tools.len(),
        2,
        "Expected exactly 2 tools, got {}",
        tools.len()
    );

    let mut names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();
    names.sort();
    assert_eq!(names, vec!["convert_time", "get_current_time"]);

    for tool in &tools {
        assert!(
            tool.description.is_some(),
            "Tool '{}' should have a description",
            tool.name
        );
        assert!(
            !tool.input_schema.is_empty(),
            "Tool '{}' should have an input schema",
            tool.name
        );
    }
}

#[tokio::test]
async fn get_current_time_returns_successful_result_via_protocol() {
    let server = TimeServer::new();
    let params = GetCurrentTimeParams { timezone: None };
    let result = server.get_current_time(Parameters(params)).await.unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(json["timezone"].is_string());
    assert!(json["datetime"].is_string());
    assert!(json["utc_offset"].is_string());
    assert!(json["is_dst"].is_boolean());
}

#[tokio::test]
async fn convert_time_returns_successful_result_via_protocol() {
    let server = TimeServer::new();
    let params = ConvertTimeParams {
        source_timezone: "UTC".into(),
        time: "12:00".into(),
        target_timezone: "Europe/London".into(),
    };
    let result = server.convert_time(Parameters(params)).await.unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["source"]["timezone"], "UTC");
    assert!(json["target"]["timezone"].is_string());
    assert!(json["target"]["datetime"].is_string());
}

#[tokio::test]
async fn get_current_time_propagates_error_for_invalid_timezone() {
    let server = TimeServer::new();
    let params = GetCurrentTimeParams {
        timezone: Some("Invalid/Timezone".into()),
    };
    let result = server.get_current_time(Parameters(params)).await.unwrap();

    assert_eq!(
        result.is_error,
        Some(true),
        "Expected is_error: true for invalid timezone"
    );
    let text = extract_text(&result);
    assert!(
        text.contains("Invalid timezone"),
        "Error message should mention invalid timezone, got: {text}"
    );
}
