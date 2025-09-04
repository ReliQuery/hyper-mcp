mod pdk;

use extism_pdk::*;
use pdk::types::{CallToolResult, Content, ContentType, ToolDescription};
use pdk::*;
use serde_json::json;
use std::error::Error as StdError;

#[derive(Debug)]
struct CustomError(String);

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for CustomError {}

// Define a compatible CallToolRequestParam structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CallToolRequestParam {
    name: String,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
}

// Host function to call tools from other plugins
#[host_fn("extism:host/user")]
extern "ExtismHost" {
    fn call_tool(request: Json<CallToolRequestParam>) -> Json<types::CallToolResult>;
}

// Called when the tool is invoked.
pub(crate) fn call(input: types::CallToolRequest) -> Result<types::CallToolResult, Error> {
    let args = input.params.arguments.unwrap_or_default();
    let name = args.get("name").unwrap().as_str().unwrap();

    match name {
        "get_wrapped_time" => {
            // Create a request to call the time::time tool with get_time_utc operation
            let cross_plugin_request = CallToolRequestParam {
                name: "time::time".to_string(), // Use time namespace
                arguments: Some({
                    let mut map = serde_json::Map::new();
                    map.insert("name".to_string(), json!("get_time_utc"));
                    map
                }),
            };

            // Call the time tool through the host function
            match unsafe { call_tool(Json(cross_plugin_request)) } {
                Ok(Json(result)) => {
                    // Wrap the response from the time plugin
                    Ok(CallToolResult {
                        content: vec![Content {
                            text: Some(
                                json!({
                                    "message": "Time retrieved via cross-plugin call",
                                    "time_data": result.content,
                                    "success": true
                                })
                                .to_string(),
                            ),
                            r#type: ContentType::Text,
                            ..Default::default()
                        }],
                        is_error: Some(false),
                    })
                }
                Err(e) => Ok(CallToolResult {
                    content: vec![Content {
                        text: Some(
                            json!({
                                "message": "Failed to call time plugin",
                                "error": format!("{:?}", e),
                                "success": false
                            })
                            .to_string(),
                        ),
                        r#type: ContentType::Text,
                        ..Default::default()
                    }],
                    is_error: Some(true),
                }),
            }
        }
        _ => Err(Error::new(CustomError("unknown command".to_string()))),
    }
}

pub(crate) fn describe() -> Result<types::ListToolsResult, Error> {
    Ok(types::ListToolsResult {
        tools: vec![ToolDescription {
            name: "wrapper".into(),
            description: "Wrapper plugin that demonstrates cross-plugin tool calls. It provides the following operations:

- `get_wrapped_time`: Calls the time plugin's get_time_utc operation through cross-plugin communication and returns the wrapped response.

This plugin is used for testing the cross_plugin_tools functionality and demonstrates how plugins can call tools from other plugins.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the operation to perform.",
                        "enum": ["get_wrapped_time"],
                    },
                },
            })
            .as_object()
            .unwrap()
            .clone(),
        }]
    })
}
