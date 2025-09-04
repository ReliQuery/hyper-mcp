# Wrapper Plugin Usage Guide

This document provides comprehensive guidance for using and testing the wrapper plugin, which demonstrates the `cross_plugin_tools` functionality in hyper-mcp.

## Overview

The wrapper plugin is a test plugin specifically designed to demonstrate cross-plugin tool calls using the new `cross_plugin_tools` configuration option. It shows how one plugin can call tools from another plugin through the `call_tool` host function exposed in the `extism:host/user` namespace.

## What It Demonstrates

- **Cross-Plugin Communication**: How plugins can call tools from other plugins
- **Host Function Usage**: Using the `call_tool` host function from `extism:host/user`
- **Namespaced Tool Calls**: Calling tools using the `plugin::tool` format (e.g., `time::time`)
- **Security Model**: Only plugins with tools listed in `cross_plugin_tools` can be called
- **Error Handling**: Proper handling of successful and failed cross-plugin calls

## Architecture

```
┌─────────────────┐    call_tool()    ┌──────────────────┐
│  Wrapper Plugin │ ───────────────► │   Time Plugin    │
│                 │                   │                  │
│ • get_wrapped_  │                   │ • time (exposed) │
│   time()        │                   │ • get_time_utc   │
│                 │                   │ • parse_time     │
│ Calls: time::   │                   │ • time_offset    │
│ time via host   │                   │                  │
│ function        │ ◄─────────────── │ Returns time data│
└─────────────────┘                   └──────────────────┘
```

## Plugin Structure

### Files Created

- `tests/integrations/wrapper-plugin/` - Main plugin directory
  - `src/lib.rs` - Main plugin implementation
  - `src/pdk.rs` - Generated PDK types and functions
  - `Cargo.toml` - Rust project configuration
  - `Dockerfile` - Container build configuration
  - `.cargo/config.toml` - Cargo build configuration
  - `README.md` - Plugin-specific documentation

### Configuration Files

- `tests/integrations/wrapper_plugin_test_config.yaml` - YAML test configuration
- `tests/integrations/wrapper_plugin_test_config.json` - JSON test configuration

## Building the Plugin

### Prerequisites

```bash
# Install Rust with WASM target
rustup target add wasm32-wasip1

# Ensure you have the required tools
cargo --version
```

### Build Steps

```bash
# Navigate to the plugin directory
cd tests/integrations/wrapper-plugin

# Build for WASM target
cargo build --release --target wasm32-wasip1

# Verify the WASM file was created
ls -la target/wasm32-wasip1/release/wrapper_plugin.wasm
```

### Using Docker

```bash
cd tests/integrations/wrapper-plugin
docker build -t wrapper-plugin .
```

## Configuration

### Basic Configuration

The wrapper plugin requires a configuration where:

1. **Time Plugin** exposes its tools via `cross_plugin_tools`
2. **Wrapper Plugin** can call those exposed tools

```yaml
plugins:
  time:
    url: "oci://ghcr.io/tuananh/time-plugin:latest"
    runtime_config:
      cross_plugin_tools:
        - "time"  # Expose the time tool for cross-plugin calls
      memory_limit: "128MB"

  wrapper:
    url: "file:///tests/integrations/wrapper-plugin/target/wasm32-wasip1/release/wrapper_plugin.wasm"
    runtime_config:
      memory_limit: "64MB"
```

### Complete Test Configuration

See `wrapper_plugin_test_config.yaml` for a comprehensive example including:

- Multiple plugin instances
- Different memory limits
- Environment variables
- Mixed cross-plugin and private tools

## Usage

### Tool Available

The wrapper plugin exposes one tool:

- **Tool Name**: `wrapper`
- **Operation**: `get_wrapped_time`
- **Description**: Calls the time plugin's `get_time_utc` operation and returns wrapped response

### Calling the Tool

```bash
# Using hyper-mcp CLI (hypothetical)
hyper-mcp call-tool wrapper::wrapper '{"name": "get_wrapped_time"}'
```

### Expected Response

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"message\":\"Time retrieved via cross-plugin call\",\"time_data\":[{\"text\":\"{\\\"utc_time\\\":\\\"1699123456\\\",\\\"utc_time_rfc2822\\\":\\\"Sun, 05 Nov 2023 10:30:56 +0000\\\"}\",\"type\":\"text\"}],\"success\":true}"
    }
  ],
  "is_error": false
}
```

## Testing

### Automated Tests

```bash
# Run configuration loading tests
cargo test test_load_wrapper_plugin_test_config

# Run all cross-plugin tools tests
cargo test cross_plugin_tools

# Run the comprehensive test script
./tests/fixtures/test_wrapper_plugin.sh
```

### Manual Testing

1. **Build hyper-mcp**:
   ```bash
   cargo build --release
   ```

2. **Start the server with test config**:
   ```bash
   RUST_LOG=info ./target/release/hyper-mcp \
     --config tests/integrations/wrapper_plugin_test_config.yaml
   ```

3. **Call the wrapper tool** (using your MCP client):
   - Tool: `wrapper::wrapper`
   - Arguments: `{"name": "get_wrapped_time"}`

## Implementation Details

### Host Function Declaration

```rust
#[host_fn("extism:host/user")]
extern "ExtismHost" {
    fn call_tool(request: Json<CallToolRequestParam>) -> Json<types::CallToolResult>;
}
```

### Cross-Plugin Call Structure

```rust
let cross_plugin_request = CallToolRequestParam {
    name: "time::time".to_string(), // Format: plugin::tool
    arguments: Some({
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), json!("get_time_utc"));
        map
    }),
};

// Make the cross-plugin call
match unsafe { call_tool(Json(cross_plugin_request)) } {
    Ok(Json(result)) => {
        // Handle successful response
    },
    Err(e) => {
        // Handle error
    }
}
```

### Security Considerations

1. **Tool Exposure Control**: Only tools listed in `cross_plugin_tools` can be called
2. **Namespace Validation**: Tools must be called with proper plugin::tool format  
3. **Memory Limits**: Each plugin respects its configured memory limits
4. **Error Isolation**: Failures in cross-plugin calls don't crash the calling plugin

## Error Scenarios

### Tool Not Exposed

If a tool is not listed in `cross_plugin_tools`:

```
Tool {tool_name} not allowed in cross-plugin calls for {plugin_name}
```

### Plugin Not Found

If the target plugin doesn't exist:

```
Plugin {plugin_name} not found
```

### Invalid Tool Format

If the tool name format is incorrect:

```
Invalid tool name format, expected plugin::tool
```

## Development Guidelines

### Adding New Operations

To add new operations to the wrapper plugin:

1. Add the operation name to the `match` statement in `call()`
2. Implement the cross-plugin call logic
3. Update the tool description schema
4. Add test cases

### Cross-Plugin Best Practices

1. **Error Handling**: Always handle both success and error cases
2. **Input Validation**: Validate arguments before making cross-plugin calls
3. **Documentation**: Document which plugins and tools your plugin depends on
4. **Testing**: Test with both available and unavailable target plugins

## Troubleshooting

### Build Issues

- **Missing Target**: Run `rustup target add wasm32-wasip1`
- **Compilation Errors**: Ensure all dependencies are compatible with WASM
- **Binary Size**: Use release builds with optimization for smaller WASM files

### Runtime Issues

- **Host Function Not Found**: Verify hyper-mcp supports the `call_tool` host function
- **Permission Denied**: Check that target tools are listed in `cross_plugin_tools`
- **Memory Limits**: Increase memory limits if plugins are hitting constraints

### Configuration Issues

- **Invalid YAML/JSON**: Use validators to check configuration syntax
- **Path Issues**: Ensure WASM file paths are absolute and accessible
- **Plugin Dependencies**: Verify all required plugins are configured

## Advanced Usage

### Chaining Multiple Calls

```rust
// Call multiple tools in sequence
let time_result = call_tool(time_request)?;
let formatted_result = call_tool(format_request)?;
```

### Conditional Cross-Plugin Calls

```rust
// Call different plugins based on input
match operation_type {
    "time" => call_tool(time_request),
    "format" => call_tool(format_request),
    _ => return local_operation(),
}
```

### Error Recovery

```rust
// Try primary plugin, fallback to secondary
match call_tool(primary_request) {
    Ok(result) => result,
    Err(_) => call_tool(fallback_request)?,
}
```

## Contributing

To contribute improvements to the wrapper plugin:

1. Fork the repository
2. Make changes in `tests/integrations/wrapper-plugin/`
3. Run the test suite: `./tests/integrations/test_wrapper_plugin.sh`
4. Update documentation as needed
5. Submit a pull request

## Related Documentation

- [Runtime Configuration Guide](../../RUNTIME_CONFIG.md)
- [Plugin Development Guide](../../examples/plugins/README.md)
- [Cross-Plugin Tools Configuration](../../README.md#cross-plugin-tools)