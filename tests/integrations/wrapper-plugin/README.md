# Wrapper Plugin

A test plugin for demonstrating cross-plugin tool calls in hyper-mcp.

## Purpose

This plugin is designed to test the `cross_plugin_tools` functionality by calling tools from other plugins. It specifically demonstrates calling the `time` plugin's `get_time_utc` operation through cross-plugin communication.

## Features

- **get_wrapped_time**: Calls the `time::time` tool's `get_time_utc` operation and returns the wrapped response
- Demonstrates the use of the `call_tool` host function from the `extism:host/user` namespace
- Tests cross-plugin communication and tool sharing

## How It Works

1. The plugin exposes a `get_wrapped_time` tool
2. When called, it creates a cross-plugin request to `time::time` with the `get_time_utc` operation
3. Uses the `call_tool` host function to invoke the time plugin
4. Wraps the response from the time plugin with additional metadata
5. Returns the wrapped result to the caller

## Building

```bash
# Build the plugin
cargo build --release --target wasm32-wasip1

# Or using Docker
docker build -t wrapper-plugin .
```

## Configuration

To use this plugin for testing, you need to configure both the wrapper plugin and the time plugin in your hyper-mcp config, with the time plugin exposing its tools via `cross_plugin_tools`:

```yaml
plugins:
  time:
    url: "oci://ghcr.io/tuananh/time-plugin:latest"
    runtime_config:
      cross_plugin_tools:
        - "time"  # Expose the time tool for cross-plugin calls
  
  wrapper:
    url: "file:///path/to/wrapper-plugin.wasm"
```

## Testing

This plugin is primarily used in automated tests to verify:

1. Cross-plugin tool calling functionality
2. Proper namespace handling (`time::time`)
3. Host function integration (`extism:host/user::call_tool`)
4. Error handling for cross-plugin calls

## Dependencies

- `extism-pdk`: Extism Plugin Development Kit
- `serde`: Serialization framework
- `serde_json`: JSON support
- `base64-serde`: Base64 encoding support

## Cross-Plugin Communication

The plugin demonstrates the cross-plugin communication pattern:

1. **Host Function**: Uses `call_tool` from `extism:host/user` namespace
2. **Namespaced Tools**: Calls `time::time` (plugin::tool format)
3. **Request Format**: Creates proper `CallToolRequestParam` structure
4. **Response Handling**: Processes `CallToolResult` from the target plugin
5. **Error Handling**: Handles both successful and failed cross-plugin calls

This pattern can be extended to create more complex plugin ecosystems where plugins can share functionality and build upon each other's capabilities.