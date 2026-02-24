use rmcp::{
    ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Error message template for invalid timezone input.
const ERR_INVALID_TIMEZONE: &str =
    "Invalid timezone: '{}'. Please use a valid IANA timezone name (e.g., 'America/New_York').";

/// Error message template for invalid time format input.
const ERR_INVALID_TIME_FORMAT: &str =
    "Invalid time format: '{}'. Expected HH:MM in 24-hour format (e.g., '14:30').";

/// MCP server providing time-related tools.
///
/// Exposes `get_current_time` and `convert_time` as MCP tools over stdio transport.
pub struct TimeServer {
    tool_router: ToolRouter<Self>,
}

/// Parameters for the `get_current_time` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCurrentTimeParams {
    /// IANA timezone name (e.g., 'America/New_York', 'Europe/London', 'Asia/Tokyo'). Defaults to UTC.
    #[serde(default)]
    pub timezone: Option<String>,
}

/// Parameters for the `convert_time` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertTimeParams {
    /// Source IANA timezone name (e.g., 'America/New_York')
    pub source_timezone: String,
    /// Time to convert in 24-hour format (HH:MM)
    pub time: String,
    /// Target IANA timezone name (e.g., 'Europe/London')
    pub target_timezone: String,
}

/// Response payload for `get_current_time`.
#[derive(Debug, Serialize)]
struct CurrentTimeResponse {
    timezone: String,
    datetime: String,
    utc_offset: String,
    is_dst: bool,
}

/// Source or target entry in the convert_time response.
#[derive(Debug, Serialize)]
struct ConvertTimeEntry {
    timezone: String,
    datetime: String,
    utc_offset: String,
}

/// Response payload for `convert_time`.
#[derive(Debug, Serialize)]
struct ConvertTimeResponse {
    source: ConvertTimeEntry,
    target: ConvertTimeEntry,
    time_difference: String,
}

impl TimeServer {
    /// Create a new TimeServer with tool routing configured.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl TimeServer {
    /// Get the current time in a specific timezone. Defaults to UTC if no timezone is provided.
    #[tool(
        name = "get_current_time",
        description = "Get the current time in a specific timezone. Defaults to UTC if no timezone is provided."
    )]
    async fn get_current_time(
        &self,
        Parameters(params): Parameters<GetCurrentTimeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let tz_input = params.timezone.unwrap_or_default();
        let tz = if tz_input.is_empty() {
            jiff::tz::TimeZone::UTC
        } else {
            match parse_timezone(&tz_input) {
                Ok(tz) => tz,
                Err(msg) => return Ok(tool_error(msg)),
            }
        };

        let now = jiff::Zoned::now().with_time_zone(tz.clone());
        let datetime = now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string();
        let utc_offset = format_utc_offset(now.offset());

        // Determine DST status using jiff's offset info, which provides
        // authoritative DST data from the timezone database.
        let info = tz.to_offset_info(now.timestamp());
        let is_dst = info.dst().is_dst();

        let tz_name = tz.iana_name().unwrap_or("UTC").to_string();

        let response = CurrentTimeResponse {
            timezone: tz_name,
            datetime,
            utc_offset,
            is_dst,
        };

        let json = serde_json::to_string_pretty(&response).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to serialize response: {e}"), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Convert a time from one timezone to another.
    #[tool(
        name = "convert_time",
        description = "Convert a time from one timezone to another."
    )]
    async fn convert_time(
        &self,
        Parameters(params): Parameters<ConvertTimeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let source_tz = match parse_timezone(&params.source_timezone) {
            Ok(tz) => tz,
            Err(msg) => return Ok(tool_error(msg)),
        };

        let target_tz = match parse_timezone(&params.target_timezone) {
            Ok(tz) => tz,
            Err(msg) => return Ok(tool_error(msg)),
        };

        let trimmed_time = params.time.trim();

        // Strict HH:MM format: reject anything that doesn't match exactly 5 chars (NN:NN)
        if trimmed_time.len() != 5 || trimmed_time.as_bytes().get(2) != Some(&b':') {
            return Ok(tool_error(ERR_INVALID_TIME_FORMAT.replacen(
                "{}",
                trimmed_time,
                1,
            )));
        }

        let time = match jiff::civil::Time::strptime("%H:%M", trimmed_time) {
            Ok(t) => t,
            Err(_) => {
                return Ok(tool_error(ERR_INVALID_TIME_FORMAT.replacen(
                    "{}",
                    trimmed_time,
                    1,
                )));
            }
        };

        // Use today's date in the source timezone
        let today = jiff::Zoned::now().with_time_zone(source_tz.clone());
        let date =
            jiff::civil::Date::new(today.year(), today.month(), today.day()).map_err(|e| {
                rmcp::ErrorData::internal_error(format!("Failed to create date: {e}"), None)
            })?;
        let datetime = date.at(time.hour(), time.minute(), 0, 0);

        let source_zdt = match datetime.to_zoned(source_tz.clone()) {
            Ok(zdt) => zdt,
            Err(_) => {
                return Ok(tool_error(format!(
                    "The time {} does not exist in timezone '{}' due to a DST transition (spring forward). \
                     Please choose a different time.",
                    trimmed_time, params.source_timezone
                )));
            }
        };

        let target_zdt = source_zdt.with_time_zone(target_tz.clone());

        let source_offset_secs = source_zdt.offset().seconds();
        let target_offset_secs = target_zdt.offset().seconds();
        let diff_secs = target_offset_secs - source_offset_secs;
        let time_difference = format_offset_diff(diff_secs);

        let source_tz_name = source_tz.iana_name().unwrap_or("UTC").to_string();
        let target_tz_name = target_tz.iana_name().unwrap_or("UTC").to_string();

        let response = ConvertTimeResponse {
            source: ConvertTimeEntry {
                timezone: source_tz_name,
                datetime: source_zdt.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
                utc_offset: format_utc_offset(source_zdt.offset()),
            },
            target: ConvertTimeEntry {
                timezone: target_tz_name,
                datetime: target_zdt.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
                utc_offset: format_utc_offset(target_zdt.offset()),
            },
            time_difference,
        };

        let json = serde_json::to_string_pretty(&response).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to serialize response: {e}"), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for TimeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mcp-time".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "A time server providing current time lookup and timezone conversion tools.".into(),
            ),
        }
    }
}

/// Construct a `CallToolResult` representing an input validation error.
///
/// Sets `is_error` to `true` and wraps the message as text content.
fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg)])
}

/// Parse and validate an IANA timezone string.
///
/// Returns an error for timezone abbreviations (e.g., "EST") and raw UTC
/// offset strings (e.g., "+05:30", "UTC+5") with a message suggesting the
/// IANA equivalent.
fn parse_timezone(input: &str) -> Result<jiff::tz::TimeZone, String> {
    // Reject raw offset strings like "+05:30", "-05:00"
    if input.starts_with('+') || input.starts_with('-') {
        return Err(format!(
            "Timezone offset '{}' is not supported. Please use a valid IANA timezone name (e.g., 'Asia/Kolkata' instead of '+05:30').",
            input
        ));
    }

    // Reject "UTC+N" or "UTC-N" style offsets
    if input.starts_with("UTC+")
        || input.starts_with("UTC-")
        || input.starts_with("GMT+")
        || input.starts_with("GMT-")
    {
        return Err(format!(
            "Timezone offset '{}' is not supported. Please use a valid IANA timezone name (e.g., 'Asia/Kolkata' instead of 'UTC+5:30').",
            input
        ));
    }

    jiff::tz::TimeZone::get(input).map_err(|_| {
        // Check if it looks like an abbreviation (all uppercase, short)
        if input.len() <= 5 && input.chars().all(|c| c.is_ascii_uppercase()) {
            format!(
                "Timezone abbreviation '{}' is ambiguous. Please use a valid IANA timezone name (e.g., 'America/New_York' instead of 'EST').",
                input
            )
        } else {
            ERR_INVALID_TIMEZONE.replacen("{}", input, 1)
        }
    })
}

/// Format a UTC offset as "+HH:MM" or "-HH:MM".
///
/// Correctly handles fractional-hour offsets (e.g., +05:45 for Asia/Kathmandu).
fn format_utc_offset(offset: jiff::tz::Offset) -> String {
    let total_seconds = offset.seconds();
    let sign = if total_seconds < 0 { '-' } else { '+' };
    let abs_seconds = total_seconds.unsigned_abs();
    let hours = abs_seconds / 3600;
    let minutes = (abs_seconds % 3600) / 60;
    format!("{}{:02}:{:02}", sign, hours, minutes)
}

/// Format an offset difference in seconds as a "+H:MM" or "-H:MM" string.
fn format_offset_diff(diff_secs: i32) -> String {
    let sign = if diff_secs < 0 { '-' } else { '+' };
    let abs = diff_secs.unsigned_abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    format!("{}{}:{:02}", sign, hours, minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timezone_returns_ok_for_valid_iana_name() {
        let result = parse_timezone("America/New_York");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_timezone_returns_err_for_invalid_name() {
        let result = parse_timezone("Fake/Zone");
        assert!(result.is_err());
    }

    #[test]
    fn parse_timezone_returns_err_for_abbreviation() {
        let result = parse_timezone("PST");
        let err = result.unwrap_err();
        assert!(err.contains("IANA timezone name"), "Error was: {err}");
    }

    #[test]
    fn parse_timezone_returns_err_for_offset_string() {
        let result = parse_timezone("+05:30");
        let err = result.unwrap_err();
        assert!(err.contains("IANA timezone name"), "Error was: {err}");
    }

    #[test]
    fn format_utc_offset_formats_positive_whole_hours() {
        let offset = jiff::tz::Offset::from_seconds(5 * 3600).unwrap();
        assert_eq!(format_utc_offset(offset), "+05:00");
    }

    #[test]
    fn format_utc_offset_formats_negative_whole_hours() {
        let offset = jiff::tz::Offset::from_seconds(-5 * 3600).unwrap();
        assert_eq!(format_utc_offset(offset), "-05:00");
    }

    #[test]
    fn format_utc_offset_formats_fractional_hours() {
        // +05:45 for Asia/Kathmandu
        let offset = jiff::tz::Offset::from_seconds(5 * 3600 + 45 * 60).unwrap();
        assert_eq!(format_utc_offset(offset), "+05:45");
    }

    #[test]
    fn format_utc_offset_formats_zero_as_positive() {
        let offset = jiff::tz::Offset::from_seconds(0).unwrap();
        assert_eq!(format_utc_offset(offset), "+00:00");
    }

    #[test]
    fn tool_error_sets_is_error_flag() {
        let result = tool_error("something went wrong");
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn tool_error_includes_message_in_content() {
        let result = tool_error("something went wrong");
        let text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => &t.text,
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("something went wrong"));
    }

    #[tokio::test]
    async fn get_current_time_defaults_to_utc_when_no_timezone() {
        let server = TimeServer::new();
        let params = GetCurrentTimeParams { timezone: None };
        let result = server.get_current_time(Parameters(params)).await.unwrap();
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["timezone"], "UTC");
    }

    #[tokio::test]
    async fn get_current_time_defaults_to_utc_for_empty_string() {
        let server = TimeServer::new();
        let params = GetCurrentTimeParams {
            timezone: Some(String::new()),
        };
        let result = server.get_current_time(Parameters(params)).await.unwrap();
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["timezone"], "UTC");
    }

    #[tokio::test]
    async fn get_current_time_returns_valid_response_for_known_timezone() {
        let server = TimeServer::new();
        let params = GetCurrentTimeParams {
            timezone: Some("America/New_York".into()),
        };
        let result = server.get_current_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["timezone"], "America/New_York");
        assert!(json["datetime"].is_string());
        assert!(json["utc_offset"].is_string());
        assert!(json["is_dst"].is_boolean());
    }

    #[tokio::test]
    async fn get_current_time_returns_error_for_invalid_timezone() {
        let server = TimeServer::new();
        let params = GetCurrentTimeParams {
            timezone: Some("Not/A/Timezone".into()),
        };
        let result = server.get_current_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("Invalid timezone"));
    }

    #[tokio::test]
    async fn get_current_time_returns_fractional_offset_for_kathmandu() {
        let server = TimeServer::new();
        let params = GetCurrentTimeParams {
            timezone: Some("Asia/Kathmandu".into()),
        };
        let result = server.get_current_time(Parameters(params)).await.unwrap();
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["utc_offset"], "+05:45");
    }

    #[tokio::test]
    async fn convert_time_converts_utc_to_new_york() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "UTC".into(),
            time: "12:00".into(),
            target_timezone: "America/New_York".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["source"]["timezone"], "UTC");
        assert_eq!(json["target"]["timezone"], "America/New_York");
        // NY is UTC-5 or UTC-4 depending on DST
        let target_dt = json["target"]["datetime"].as_str().unwrap();
        assert!(
            target_dt.contains("07:00") || target_dt.contains("08:00"),
            "Expected 07:00 or 08:00 but got: {target_dt}"
        );
    }

    #[tokio::test]
    async fn convert_time_returns_error_for_invalid_time_format() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "UTC".into(),
            time: "25:99".into(),
            target_timezone: "America/New_York".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("Invalid time format"));
    }

    #[tokio::test]
    async fn convert_time_returns_error_for_invalid_source_timezone() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "Bad/Zone".into(),
            time: "12:00".into(),
            target_timezone: "UTC".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("Invalid timezone"));
    }

    #[tokio::test]
    async fn convert_time_trims_whitespace_from_time_input() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "UTC".into(),
            time: "  14:30  ".into(),
            target_timezone: "UTC".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn convert_time_rejects_24_00() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "UTC".into(),
            time: "24:00".into(),
            target_timezone: "UTC".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("Invalid time format"));
    }

    #[tokio::test]
    async fn convert_time_rejects_time_with_seconds() {
        let server = TimeServer::new();
        let params = ConvertTimeParams {
            source_timezone: "UTC".into(),
            time: "14:30:00".into(),
            target_timezone: "UTC".into(),
        };
        let result = server.convert_time(Parameters(params)).await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("Invalid time format"));
    }

    /// Extract text content from a CallToolResult.
    fn extract_text(result: &CallToolResult) -> String {
        match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        }
    }
}
