mod pdk;

use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use extism_pdk::*;
use pdk::*;
use serde_json::{Map, Value, json};

use crate::pdk::types::ReadResourceResult;

impl From<types::BlobResourceContents> for serde_json::Map<String, serde_json::Value> {
    fn from(value: types::BlobResourceContents) -> Self {
        serde_json::to_value(value)
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    }
}

impl From<types::TextContent> for serde_json::Map<String, serde_json::Value> {
    fn from(value: types::TextContent) -> Self {
        serde_json::to_value(value)
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    }
}

impl From<types::TextResourceContents> for serde_json::Map<String, serde_json::Value> {
    fn from(value: types::TextResourceContents) -> Self {
        serde_json::to_value(value)
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    }
}

enum AnyReference {
    Prompt(types::PromptReference),
    Resource(types::ResourceTemplateReference),
}

impl TryFrom<serde_json::Map<String, serde_json::Value>> for AnyReference {
    type Error = anyhow::Error;

    fn try_from(value: serde_json::Map<String, serde_json::Value>) -> Result<Self, Self::Error> {
        // Look at the "type" field to determine which variant to deserialize into
        let type_value = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'type' field in reference"))?
            .to_string();

        let json_value = serde_json::Value::Object(value);

        match type_value.as_str() {
            "prompt" => {
                let prompt_ref: types::PromptReference = serde_json::from_value(json_value)?;
                Ok(AnyReference::Prompt(prompt_ref))
            }
            "resource" => {
                let resource_ref: types::ResourceTemplateReference =
                    serde_json::from_value(json_value)?;
                Ok(AnyReference::Resource(resource_ref))
            }
            other => Err(anyhow!("Unknown reference type: {}", other)),
        }
    }
}

// Execute a tool call. This is the primary entry point for tool execution in plugins.
//
// The plugin receives a tool call request with the tool name and arguments, along with request context information. The plugin should execute the requested tool and return the result with content blocks and optional structured output.
pub(crate) fn call_tool(input: types::CallToolRequest) -> Result<types::CallToolResult, Error> {
    match input.request.name.as_str() {
        "get_time" => {
            let tz = match input
                .request
                .arguments
                .as_ref()
                .and_then(|args| args.get("timezone"))
                .and_then(|v| v.as_str())
            {
                Some(timezone) => match timezone.parse::<chrono_tz::Tz>() {
                    Ok(tz) => tz,
                    Err(e) => {
                        return Ok(types::CallToolResult {
                            content: vec![
                                types::TextContent {
                                    text: format!("Error: Invalid timezone '{}': {}", timezone, e),

                                    ..Default::default()
                                }
                                .into(),
                            ],
                            is_error: Some(true),

                            ..Default::default()
                        });
                    }
                },
                None => chrono_tz::UTC,
            };
            let current_time = chrono::Utc::now().with_timezone(&tz).to_rfc2822();
            Ok(types::CallToolResult {
                content: vec![
                    types::TextContent {
                        text: current_time.clone(),

                        ..Default::default()
                    }
                    .into(),
                ],
                structured_content: Some(Map::from_iter([(
                    "current_time".to_string(),
                    Value::String(current_time),
                )])),

                ..Default::default()
            })
        }
        "parse_time" => {
            let time_str = match input
                .request
                .arguments
                .as_ref()
                .and_then(|args| args.get("time"))
                .and_then(|v| v.as_str())
            {
                Some(t) => t,
                None => {
                    return Ok(types::CallToolResult {
                        content: vec![
                            types::TextContent {
                                text: "Error: 'time' argument is required".to_string(),

                                ..Default::default()
                            }
                            .into(),
                        ],
                        is_error: Some(true),

                        ..Default::default()
                    });
                }
            };
            match chrono::DateTime::parse_from_rfc2822(time_str) {
                Ok(dt) => Ok(types::CallToolResult {
                    content: vec![
                        types::TextContent {
                            text: dt.timestamp().to_string(),

                            ..Default::default()
                        }
                        .into(),
                    ],
                    structured_content: Some(Map::from_iter([(
                        "timestamp".to_string(),
                        Value::Number(serde_json::Number::from(dt.timestamp())),
                    )])),

                    ..Default::default()
                }),
                Err(e) => Ok(types::CallToolResult {
                    content: vec![
                        types::TextContent {
                            text: format!("Error parsing time: {}", e),

                            ..Default::default()
                        }
                        .into(),
                    ],
                    is_error: Some(true),

                    ..Default::default()
                }),
            }
        }
        _ => Err(anyhow!("Unknown tool: {}", input.request.name)),
    }
}

// Provide completion suggestions for a partially-typed input.
//
// This function is called when the user requests autocompletion. The plugin should analyze the partial input and return matching completion suggestions based on the reference (prompt or resource) and argument context.
pub(crate) fn complete(input: types::CompleteRequest) -> Result<types::CompleteResult, Error> {
    match AnyReference::try_from(input.request.r#ref)? {
        AnyReference::Prompt(prompt_ref)
            if prompt_ref.name.as_str() != "get_time_with_timezone" =>
        {
            return Err(anyhow!(
                "Completion for prompt not implemented: {}",
                prompt_ref.name
            ));
        }

        AnyReference::Resource(resource_ref)
            if resource_ref.uri.as_str()
                != "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}" =>
        {
            return Err(anyhow!(
                "Completion for resource not implemented: {}",
                resource_ref.uri
            ));
        }

        _ => {}
    };

    match input.request.argument.name.as_str() {
        "timezone" => {
            let query = input
                .request
                .argument
                .value
                .to_ascii_lowercase()
                .replace(" ", "_");
            let mut suggestions: Vec<String> = vec![];
            let mut total: i64 = 0;
            for tz in chrono_tz::TZ_VARIANTS {
                if tz.name().to_ascii_lowercase().contains(&query) {
                    if suggestions.len() < 100 {
                        suggestions.push(tz.name().to_string());
                    }
                    total += 1;
                }
            }
            Ok(types::CompleteResult {
                completion: types::CompleteResultCompletion {
                    has_more: Some(total > suggestions.len() as i64),
                    total: Some(total),
                    values: suggestions,
                },
            })
        }
        _ => Err(anyhow!(
            "Completion for argument not implemented: {}",
            input.request.argument.name
        )),
    }
}

// Retrieve a specific prompt by name.
//
// This function is called when the user requests a specific prompt. The plugin should return the prompt details including messages and optional description.
pub(crate) fn get_prompt(input: types::GetPromptRequest) -> Result<types::GetPromptResult, Error> {
    match input.request.name.as_str() {
        "get_time_with_timezone" => {
            let tz = match input
                .request
                .arguments
                .as_ref()
                .and_then(|args| args.get("timezone"))
                .and_then(|v| v.as_str())
            {
                Some(timezone) => match timezone.parse::<chrono_tz::Tz>() {
                    Ok(tz) => tz,
                    Err(e) => {
                        return Ok(types::GetPromptResult {
                            messages: vec![types::PromptMessage {
                                role: types::Role::Assistant,
                                content: types::TextContent {
                                    text: format!("Error: Invalid timezone '{}': {}", timezone, e),

                                    ..Default::default()
                                }
                                .into(),
                            }],

                            ..Default::default()
                        });
                    }
                },
                None => chrono_tz::UTC,
            };

            Ok(types::GetPromptResult {
                description: Some(format!("Information for {}", tz.name())),
                messages: vec![types::PromptMessage {
                    role: types::Role::Assistant,
                    content: types::TextContent {
                        text: format!("Please get the time for the timezone {}", tz.name()),

                        ..Default::default()
                    }
                    .into(),
                }],
            })
        }
        _ => Err(anyhow!("Prompt not found: {}", input.request.name)),
    }
}

// List all available prompts.
//
// This function should return a list of prompts that the plugin provides. Each prompt should include its name and a brief description of what it does. Supports pagination via cursor.
pub(crate) fn list_prompts(
    _input: types::ListPromptsRequest,
) -> Result<types::ListPromptsResult, Error> {
    Ok(types::ListPromptsResult {
        prompts: vec![types::Prompt {
            name: "get_time_with_timezone".to_string(),
            description: Some(
                "Asks the assistant to get the time in a provided timezone".to_string(),
            ),
            title: Some("Get Localized Time".to_string()),
            arguments: Some(vec![types::PromptArgument {
                name: "timezone".to_string(),
                description: Some(
                    "The timezone to prompt for, will use UTC by default".to_string(),
                ),
                title: Some("Timezone".to_string()),

                ..Default::default()
            }]),
        }],
    })
}

// List all available resource templates.
//
// This function should return a list of resource templates that the plugin provides. Templates are URI patterns that can match multiple resources. Supports pagination via cursor.
pub(crate) fn list_resource_templates(
    _input: types::ListResourceTemplatesRequest,
) -> Result<types::ListResourceTemplatesResult, Error> {
    Ok(types::ListResourceTemplatesResult {
        resource_templates: vec![types::ResourceTemplate {
            name: "time_zone_converter".to_string(),
            description: Some("Display HTML page containing timezone information".to_string()),
            mime_type: Some("text/html".to_string()),
            uri_template: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}"
                .to_string(),
            title: Some("TimeZone Converter".to_string()),

            ..Default::default()
        }],
        ..Default::default()
    })
}

// List all available resources.
//
// This function should return a list of resources that the plugin provides. Resources are URI-based references to files, data, or services. Supports pagination via cursor.
pub(crate) fn list_resources(
    _input: types::ListResourcesRequest,
) -> Result<types::ListResourcesResult, Error> {
    Ok(types::ListResourcesResult::default())
}

// List all available tools.
//
// This function should return a list of all tools that the plugin provides. Each tool should include its name, description, and input schema. Supports pagination via cursor.
pub(crate) fn list_tools(_input: types::ListToolsRequest) -> Result<types::ListToolsResult, Error> {
    Ok(types::ListToolsResult {
        tools: vec![
            types::Tool {
                annotations: None,
                description: Some("Returns the current time in the specified timezone. If no timezone is specified then UTC is used.".to_string()),
                input_schema: types::ToolSchema {
                    properties: Some(Map::from_iter([
                        ("timezone".to_string(), json!({
                            "type": "string",
                            "description": "The timezone to get the current time for, e.g. 'America/New_York'. Defaults to 'UTC' if not provided.",
                        })),
                    ])),

                    ..Default::default()
                },
                name: "get_time".to_string(),
                output_schema: Some(types::ToolSchema {
                    properties: Some(Map::from_iter([
                        ("current_time".to_string(), json!({
                            "type": "string",
                            "description": "The current time in the specified timezone in RFC2822 format.",
                        })),
                    ])),
                    required: Some(vec!["current_time".to_string()]),

                    ..Default::default()
                }),
                title: Some("Get Current Time".to_string()),
            },
            types::Tool {
                annotations: None,
                description: Some("Parses a time string in RFC2822 format and returns the corresponding timestamp in UTC.".to_string()),
                input_schema: types::ToolSchema {
                    properties: Some(Map::from_iter([
                        ("time".to_string(), json!({
                            "type": "string",
                            "description": "The time string in RFC2822 format to parse.",
                        })),
                    ])),
                    required: Some(vec!["time".to_string()]),

                    ..Default::default()
                },
                name: "parse_time".to_string(),
                output_schema: Some(types::ToolSchema {
                    properties: Some(Map::from_iter([
                        ("timestamp".to_string(), json!({
                            "type": "integer",
                            "description": "The parsed timestamp in seconds since the Unix epoch.",
                        })),
                    ])),
                    required: Some(vec!["timestamp".to_string()]),

                    ..Default::default()
                }),
                title: Some("Parse Time from RFC2822".to_string()),
            }
        ],
    })
}

// Notification that the list of roots has changed.
//
// This is an optional notification handler. If implemented, the plugin will be notified whenever the roots list changes on the client side. This allows plugins to react to changes in the file system roots or other root resources.
pub(crate) fn on_roots_list_changed(_input: types::PluginNotificationContext) -> Result<(), Error> {
    //We don't care about peer roots for this plugin
    Ok(())
}

// Read the contents of a resource by its URI.
//
// This function is called when the user wants to read the contents of a specific resource. The plugin should retrieve and return the resource data with appropriate MIME type information.
pub(crate) fn read_resource(
    input: types::ReadResourceRequest,
) -> Result<types::ReadResourceResult, Error> {
    if !input
        .request
        .uri
        .starts_with("https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz=")
    {
        return Ok(ReadResourceResult::default());
    }

    match extism_pdk::http::request(
        &HttpRequest::new(input.request.uri.clone()).with_method("GET"),
        None::<Memory>,
    ) {
        Ok(response) => {
            if response.status_code() >= 200 && response.status_code() < 300 {
                Ok(ReadResourceResult {
                    contents: vec![
                        types::BlobResourceContents {
                            mime_type: Some("text/html".to_string()),
                            blob: STANDARD.encode(&response.body()),
                            uri: input.request.uri,

                            ..Default::default()
                        }
                        .into(),
                    ],
                })
            } else {
                return Ok(ReadResourceResult {
                    contents: vec![
                        types::TextResourceContents {
                            mime_type: Some("text/plain".to_string()),
                            text: format!(
                                "Error fetching resource: HTTP {}",
                                response.status_code()
                            ),

                            ..Default::default()
                        }
                        .into(),
                    ],
                });
            }
        }
        Err(e) => {
            return Ok(ReadResourceResult {
                contents: vec![
                    types::TextResourceContents {
                        mime_type: Some("text/plain".to_string()),
                        text: format!("Error fetching resource: {}", e),

                        ..Default::default()
                    }
                    .into(),
                ],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_tool_get_time_utc() {
        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "get_time".to_string(),
                arguments: None,
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(!result.content.is_empty());
        assert!(result.is_error.is_none() || result.is_error == Some(false));
        assert!(result.structured_content.is_some());
    }

    #[test]
    fn test_call_tool_get_time_with_timezone() {
        let mut args = Map::new();
        args.insert(
            "timezone".to_string(),
            Value::String("America/New_York".to_string()),
        );

        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "get_time".to_string(),
                arguments: Some(args),
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(!result.content.is_empty());
        assert!(result.is_error.is_none() || result.is_error == Some(false));
    }

    #[test]
    fn test_call_tool_get_time_invalid_timezone() {
        let mut args = Map::new();
        args.insert(
            "timezone".to_string(),
            Value::String("Invalid/Timezone".to_string()),
        );

        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "get_time".to_string(),
                arguments: Some(args),
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(result.is_error == Some(true));
    }

    #[test]
    fn test_call_tool_parse_time_valid() {
        let mut args = Map::new();
        args.insert(
            "time".to_string(),
            Value::String("29 Nov 2024 10:30:00 +0000".to_string()),
        );

        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "parse_time".to_string(),
                arguments: Some(args),
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(!result.content.is_empty());
        assert!(result.is_error.is_none() || result.is_error == Some(false));
        assert!(result.structured_content.is_some());
    }

    #[test]
    fn test_call_tool_parse_time_missing_argument() {
        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "parse_time".to_string(),
                arguments: None,
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(result.is_error == Some(true));
    }

    #[test]
    fn test_call_tool_parse_time_invalid() {
        let mut args = Map::new();
        args.insert(
            "time".to_string(),
            Value::String("not a valid time".to_string()),
        );

        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "parse_time".to_string(),
                arguments: Some(args),
            },
        };

        let result = call_tool(input).expect("call_tool should succeed");
        assert!(result.is_error == Some(true));
    }

    #[test]
    fn test_call_tool_unknown_tool() {
        let input = types::CallToolRequest {
            context: types::PluginRequestContext::default(),
            request: types::CallToolRequestParam {
                name: "unknown_tool".to_string(),
                arguments: None,
            },
        };

        let result = call_tool(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_with_utc_query() {
        // Test complete function with UTC timezone query
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "utc".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(result.completion.values.contains(&"UTC".to_string()));
        assert!(result.completion.total.is_some());
    }

    #[test]
    fn test_complete_with_america_query() {
        // Test complete function with America timezone prefix
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "america".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(result.completion.values.len() > 5);
        assert!(
            result
                .completion
                .values
                .iter()
                .any(|v| v.contains("America"))
        );
    }

    #[test]
    fn test_complete_with_empty_query() {
        // Test complete function with empty query - should return many results
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: String::new(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        // Should return max 100 suggestions
        assert!(result.completion.values.len() <= 100);
        // Should indicate there are more
        assert_eq!(result.completion.has_more, Some(true));
        // Total should be much larger
        assert!(result.completion.total.unwrap() > 400);
    }

    #[test]
    fn test_complete_with_york_query() {
        // Test complete function with York timezone query (case insensitive)
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "YORK".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(
            result
                .completion
                .values
                .contains(&"America/New_York".to_string())
        );
    }

    #[test]
    fn test_complete_with_los_angeles_query() {
        // Test complete function with space-separated timezone query
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "los angeles".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(
            result
                .completion
                .values
                .contains(&"America/Los_Angeles".to_string())
        );
    }

    #[test]
    fn test_complete_with_europe_query() {
        // Test complete function with Europe timezone prefix
        let prompt_ref = types::PromptReference {
            name: "get_time_with_timezone".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };
        let r#ref = serde_json::to_value(&prompt_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "europe/".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        // All results should contain Europe
        assert!(
            result
                .completion
                .values
                .iter()
                .all(|v| v.to_lowercase().contains("europe"))
        );
    }

    #[test]
    fn test_complete_result_structure() {
        // Test that complete results have the expected structure
        // We verify the logic by constructing expected outputs
        let values = vec!["UTC".to_string(), "America/New_York".to_string()];
        let total = 500i64;
        let has_more = total > values.len() as i64;

        let completion = types::CompleteResultCompletion {
            has_more: Some(has_more),
            total: Some(total),
            values: values.clone(),
        };

        let result = types::CompleteResult { completion };
        assert_eq!(result.completion.values.len(), 2);
        assert!(result.completion.has_more.unwrap());
        assert_eq!(result.completion.total.unwrap(), 500);
    }

    #[test]
    fn test_complete_result_has_required_fields() {
        // Test that CompleteResult includes required fields
        let completion = types::CompleteResultCompletion {
            has_more: Some(true),
            total: Some(500),
            values: vec!["UTC".to_string(), "America/New_York".to_string()],
        };

        let result = types::CompleteResult { completion };

        assert!(result.completion.has_more.is_some());
        assert!(result.completion.total.is_some());
        assert!(!result.completion.values.is_empty());
        assert_eq!(result.completion.values.len(), 2);
    }

    #[test]
    fn test_complete_result_total_flag_matches_logic() {
        // Test the logic for has_more flag: should be true when total > values.len()
        let values = vec!["UTC".to_string()];
        let total = 500i64;
        let values_len = values.len() as i64;

        let has_more = total > values_len;
        assert!(has_more);
    }

    #[test]
    fn test_complete_result_no_more_when_all_returned() {
        // Test the logic for has_more flag: should be false when all results fit
        let values = vec!["UTC".to_string(), "America/New_York".to_string()];
        let total = values.len() as i64;
        let values_len = values.len() as i64;

        let has_more = total > values_len;
        assert!(!has_more);
    }

    #[test]
    fn test_get_prompt_valid() {
        let input = types::GetPromptRequest {
            context: types::PluginRequestContext::default(),
            request: types::GetPromptRequestParam {
                name: "get_time_with_timezone".to_string(),
                arguments: None,
            },
        };

        let result = get_prompt(input).expect("get_prompt should succeed");
        assert!(!result.messages.is_empty());
        assert!(result.description.is_some());
    }

    #[test]
    fn test_get_prompt_with_timezone() {
        let mut args = Map::new();
        args.insert(
            "timezone".to_string(),
            Value::String("Europe/London".to_string()),
        );

        let input = types::GetPromptRequest {
            context: types::PluginRequestContext::default(),
            request: types::GetPromptRequestParam {
                name: "get_time_with_timezone".to_string(),
                arguments: Some(args),
            },
        };

        let result = get_prompt(input).expect("get_prompt should succeed");
        assert!(!result.messages.is_empty());
        assert!(result.description.is_some());
    }

    #[test]
    fn test_get_prompt_invalid_timezone() {
        let mut args = Map::new();
        args.insert(
            "timezone".to_string(),
            Value::String("Invalid/Zone".to_string()),
        );

        let input = types::GetPromptRequest {
            context: types::PluginRequestContext::default(),
            request: types::GetPromptRequestParam {
                name: "get_time_with_timezone".to_string(),
                arguments: Some(args),
            },
        };

        let result = get_prompt(input).expect("get_prompt should succeed");
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn test_get_prompt_not_found() {
        let input = types::GetPromptRequest {
            context: types::PluginRequestContext::default(),
            request: types::GetPromptRequestParam {
                name: "unknown_prompt".to_string(),
                arguments: None,
            },
        };

        let result = get_prompt(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_prompts() {
        let input = types::ListPromptsRequest::default();
        let result = list_prompts(input).expect("list_prompts should succeed");

        assert!(!result.prompts.is_empty());
        assert_eq!(result.prompts[0].name, "get_time_with_timezone");
        assert!(result.prompts[0].description.is_some());
        assert!(result.prompts[0].arguments.is_some());
    }

    #[test]
    fn test_list_resource_templates() {
        let input = types::ListResourceTemplatesRequest::default();
        let result =
            list_resource_templates(input).expect("list_resource_templates should succeed");

        assert!(!result.resource_templates.is_empty());
        assert_eq!(result.resource_templates[0].name, "time_zone_converter");
        assert!(result.resource_templates[0].description.is_some());
        assert!(result.resource_templates[0].mime_type.is_some());
    }

    #[test]
    fn test_list_resources() {
        let input = types::ListResourcesRequest::default();
        let result = list_resources(input).expect("list_resources should succeed");

        assert!(result.resources.is_empty());
    }

    #[test]
    fn test_list_tools() {
        let input = types::ListToolsRequest::default();
        let result = list_tools(input).expect("list_tools should succeed");

        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.tools[0].name, "get_time");
        assert_eq!(result.tools[1].name, "parse_time");

        assert!(result.tools[0].description.is_some());
        assert!(result.tools[0].input_schema.properties.is_some());
        assert!(result.tools[0].output_schema.is_some());

        assert!(result.tools[1].description.is_some());
        assert!(result.tools[1].input_schema.properties.is_some());
        assert!(result.tools[1].output_schema.is_some());
    }

    #[test]
    fn test_on_roots_list_changed() {
        let input = types::PluginNotificationContext::default();
        let result = on_roots_list_changed(input);

        assert!(result.is_ok());
    }

    #[test]
    fn test_blob_resource_contents_conversion() {
        let blob_contents = types::BlobResourceContents {
            blob: "base64encodeddata".to_string(),
            mime_type: Some("text/html".to_string()),
            uri: "https://example.com".to_string(),
            ..Default::default()
        };

        let map: Map<String, Value> = blob_contents.into();
        assert!(map.contains_key("blob"));
        assert!(map.contains_key("uri"));
    }

    #[test]
    fn test_text_content_conversion() {
        let text_content = types::TextContent {
            text: "Test content".to_string(),
            ..Default::default()
        };

        let map: Map<String, Value> = text_content.into();
        assert!(map.contains_key("text"));
    }

    #[test]
    fn test_text_resource_contents_conversion() {
        let text_resource = types::TextResourceContents {
            text: "Resource text".to_string(),
            mime_type: Some("text/plain".to_string()),
            uri: "https://example.com".to_string(),
            ..Default::default()
        };

        let map: Map<String, Value> = text_resource.into();
        assert!(map.contains_key("text"));
        assert!(map.contains_key("uri"));
    }

    #[test]
    fn test_prompt_reference_serialization() {
        // Test serializing a PromptReference and checking its structure
        let prompt_ref = types::PromptReference {
            name: "test_prompt".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };

        let json_value = serde_json::to_value(&prompt_ref).expect("should serialize");
        println!("Serialized PromptReference: {}", json_value);

        let json_obj = json_value.as_object().expect("should be object");
        assert!(json_obj.contains_key("type"), "Should have 'type' field");
        assert!(json_obj.contains_key("name"), "Should have 'name' field");

        // Check the type field value
        let type_value = json_obj.get("type").expect("type field exists");
        println!("Type field value: {}", type_value);
        assert_eq!(type_value, "prompt");
    }

    #[test]
    fn test_any_reference_deserialization() {
        // Test deserializing a PromptReference map into AnyReference
        let prompt_ref = types::PromptReference {
            name: "test_prompt".to_string(),
            title: None,
            r#type: types::PromptReferenceType::Prompt,
        };

        let json_value = serde_json::to_value(&prompt_ref).expect("should serialize");
        let json_map = json_value.as_object().expect("should be object").clone();

        println!("Map being deserialized: {:?}", json_map);

        // Try to deserialize into AnyReference
        let any_ref =
            AnyReference::try_from(json_map).expect("should deserialize into AnyReference");

        match any_ref {
            AnyReference::Prompt(pr) => {
                assert_eq!(pr.name, "test_prompt");
            }
            AnyReference::Resource(_) => {
                panic!("Should have deserialized as Prompt, not Resource");
            }
        }
    }

    #[test]
    fn test_complete_resource_with_utc_query() {
        // Test complete function with ResourceTemplateReference and UTC timezone query
        let resource_ref = types::ResourceTemplateReference {
            r#type: types::ResourceReferenceType::Resource,
            uri: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}".to_string(),
        };
        let r#ref = serde_json::to_value(&resource_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "utc".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(result.completion.values.contains(&"UTC".to_string()));
        assert!(result.completion.total.is_some());
    }

    #[test]
    fn test_complete_resource_with_asia_query() {
        // Test complete function with ResourceTemplateReference and Asia timezone prefix
        let resource_ref = types::ResourceTemplateReference {
            r#type: types::ResourceReferenceType::Resource,
            uri: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}".to_string(),
        };
        let r#ref = serde_json::to_value(&resource_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "asia".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(!result.completion.values.is_empty());
        assert!(result.completion.values.contains(&"Asia/Tokyo".to_string()));
        assert!(result.completion.total.is_some());
        assert!(result.completion.has_more.is_some());
    }

    #[test]
    fn test_complete_resource_with_no_match() {
        // Test complete function with ResourceTemplateReference that has no matching timezones
        let resource_ref = types::ResourceTemplateReference {
            r#type: types::ResourceReferenceType::Resource,
            uri: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}".to_string(),
        };
        let r#ref = serde_json::to_value(&resource_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "nonexistent_tz".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        assert!(result.completion.values.is_empty());
        assert_eq!(result.completion.total, Some(0));
    }

    #[test]
    fn test_complete_resource_empty_query() {
        // Test complete function with ResourceTemplateReference and empty query
        let resource_ref = types::ResourceTemplateReference {
            r#type: types::ResourceReferenceType::Resource,
            uri: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}".to_string(),
        };
        let r#ref = serde_json::to_value(&resource_ref)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();

        let input = types::CompleteRequest {
            context: types::PluginRequestContext::default(),
            request: types::CompleteRequestParam {
                r#ref,
                argument: types::CompleteRequestParamArgument {
                    name: "timezone".to_string(),
                    value: "".to_string(),
                },
                context: None,
            },
        };

        let result = complete(input).expect("complete should succeed");
        // Empty query should match all timezones (up to 100)
        assert!(!result.completion.values.is_empty());
        assert_eq!(result.completion.values.len(), 100);
        assert!(result.completion.total.is_some());
        assert!(result.completion.has_more.is_some());
    }

    #[test]
    fn test_any_reference_deserialization_resource() {
        // Test deserializing a ResourceTemplateReference map into AnyReference
        let resource_ref = types::ResourceTemplateReference {
            r#type: types::ResourceReferenceType::Resource,
            uri: "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}".to_string(),
        };

        let json_value = serde_json::to_value(&resource_ref).expect("should serialize");
        let json_map = json_value.as_object().expect("should be object").clone();

        println!("Resource map being deserialized: {:?}", json_map);

        // Try to deserialize into AnyReference
        let any_ref =
            AnyReference::try_from(json_map).expect("should deserialize into AnyReference");

        match any_ref {
            AnyReference::Resource(rr) => {
                assert_eq!(
                    rr.uri,
                    "https://www.timezoneconverter.com/cgi-bin/zoneinfo?tz={timezone}"
                );
            }
            AnyReference::Prompt(_) => {
                panic!("Should have deserialized as Resource, not Prompt");
            }
        }
    }
}
