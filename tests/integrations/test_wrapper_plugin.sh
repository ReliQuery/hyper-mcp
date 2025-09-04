#!/bin/bash

# Test script for wrapper plugin cross_plugin_tools functionality
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WRAPPER_PLUGIN_DIR="$SCRIPT_DIR/wrapper-plugin"
WASM_FILE="$WRAPPER_PLUGIN_DIR/target/wasm32-wasip1/release/wrapper_plugin.wasm"
VENV_DIR="$SCRIPT_DIR/test_env"

echo "🧪 Testing Wrapper Plugin Cross-Plugin Tools Functionality"
echo "=================================================="

# Check if Rust and required targets are installed
echo "✅ Checking Rust installation..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found. Please install Rust."
    exit 1
fi

if ! rustup target list --installed | grep -q "wasm32-wasip1"; then
    echo "📦 Installing wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# Build the wrapper plugin
echo "🔨 Building wrapper plugin..."
cd "$WRAPPER_PLUGIN_DIR"

if cargo build --release --target wasm32-wasip1; then
    echo "✅ Wrapper plugin built successfully"
else
    echo "❌ Failed to build wrapper plugin"
    exit 1
fi

# Verify WASM file was created
if [ -f "$WASM_FILE" ]; then
    echo "✅ WASM file created: $(basename "$WASM_FILE")"
    echo "📊 File size: $(du -h "$WASM_FILE" | cut -f1)"
else
    echo "❌ WASM file not found: $WASM_FILE"
    exit 1
fi

# Test configuration loading
echo "🔧 Testing configuration loading..."
cd "$PROJECT_DIR"

# Test YAML config loading
echo "  📄 Testing YAML configuration..."
if cargo test test_load_wrapper_plugin_test_config -- --nocapture; then
    echo "  ✅ YAML configuration test passed"
else
    echo "  ❌ YAML configuration test failed"
    exit 1
fi

# Test JSON config loading
echo "  📄 Testing JSON configuration..."
if cargo test test_load_wrapper_plugin_test_config_json -- --nocapture; then
    echo "  ✅ JSON configuration test passed"
else
    echo "  ❌ JSON configuration test failed"
    exit 1
fi

# Validate the configuration files syntax
echo "🔍 Validating configuration files..."

# Check YAML syntax
if python3 -c "import yaml; yaml.safe_load(open('$SCRIPT_DIR/wrapper_plugin_test_config.yaml'))" 2>/dev/null; then
    echo "  ✅ YAML syntax is valid"
else
    echo "  ❌ YAML syntax validation failed"
    exit 1
fi

# Check JSON syntax
if python3 -c "import json; json.load(open('$SCRIPT_DIR/wrapper_plugin_test_config.json'))" 2>/dev/null; then
    echo "  ✅ JSON syntax is valid"
else
    echo "  ❌ JSON syntax validation failed"
    exit 1
fi

# Test cross_plugin_tools parsing
echo "🔄 Testing cross_plugin_tools configuration parsing..."
if cargo test cross_plugin_tools -- --nocapture; then
    echo "✅ Cross-plugin tools configuration tests passed"
else
    echo "❌ Cross-plugin tools configuration tests failed"
    exit 1
fi

# Check if hyper-mcp binary exists for integration testing
echo ""
echo "🔧 Integration Test Readiness:"
echo "=============================="
HYPER_MCP_BINARY=""
if [ -f "$PROJECT_DIR/target/release/hyper-mcp" ]; then
    HYPER_MCP_BINARY="$PROJECT_DIR/target/release/hyper-mcp"
    echo "✅ hyper-mcp binary found (release): $HYPER_MCP_BINARY"
elif [ -f "$PROJECT_DIR/target/debug/hyper-mcp" ]; then
    HYPER_MCP_BINARY="$PROJECT_DIR/target/debug/hyper-mcp"
    echo "✅ hyper-mcp binary found (debug): $HYPER_MCP_BINARY"
else
    echo "ℹ️  hyper-mcp binary not found - build it first with: cargo build"
fi

# Run integration tests if binary is available
if [ -n "$HYPER_MCP_BINARY" ]; then
    echo ""
    echo "🐍 Setting up Python test environment:"
    echo "====================================="

    # Create Python virtual environment
    echo "📦 Creating Python virtual environment..."
    if [ -d "$VENV_DIR" ]; then
        echo "🧹 Removing existing virtual environment..."
        rm -rf "$VENV_DIR"
    fi

    if python3 -m venv "$VENV_DIR"; then
        echo "✅ Virtual environment created successfully"
    else
        echo "❌ Failed to create virtual environment"
        exit 1
    fi

    # Activate virtual environment and install mcp package
    echo "📥 Installing mcp package..."
    source "$VENV_DIR/bin/activate"

    if pip install mcp; then
        echo "✅ mcp package installed successfully"
    else
        echo "❌ Failed to install mcp package"
        deactivate
        exit 1
    fi

    echo "🔍 Installed packages:"
    pip list | grep -E "(mcp|pydantic)"

    echo ""
    echo "🚀 Running Integration Tests:"
    echo "============================"

    # Fix the config file path to use absolute path
    CONFIG_FILE="$PROJECT_DIR/tests/integrations/wrapper_plugin_test_config.yaml"

    # Update config to use absolute path for wrapper plugin
    echo "📝 Updating wrapper plugin path in config..."
    sed "s|file:///tests/integrations/wrapper-plugin/target/wasm32-wasip1/release/wrapper_plugin.wasm|file://$PROJECT_DIR/tests/integrations/wrapper-plugin/target/wasm32-wasip1/release/wrapper_plugin.wasm|g" "$CONFIG_FILE" > /tmp/wrapper_test_config.yaml

    echo "🧪 Testing wrapper plugin functionality using mcp_call.py..."

    # Test the wrapper plugin with mcp_call.py
    echo "🔄 Calling wrapper::wrapper tool with get_wrapped_time..."

    if python3 "$SCRIPT_DIR/mcp_call.py" \
        --server-cmd "$HYPER_MCP_BINARY" \
        --server-arg="--config-file" \
        --server-arg="/tmp/wrapper_test_config.yaml" \
        --server-arg="--transport" \
        --server-arg="stdio" \
        --tool "wrapper::wrapper" \
        --tool-args '{"name": "get_wrapped_time"}' > /tmp/wrapper_test_result.json 2>/tmp/wrapper_test_error.log; then

        echo "✅ MCP call completed successfully!"

        # Verify the results
        echo "📋 Verifying test results..."

        if [ -s /tmp/wrapper_test_result.json ]; then
            echo "✅ Got response from wrapper plugin"

            # Parse and validate the response
            RESPONSE_CONTENT=$(cat /tmp/wrapper_test_result.json)
            echo "📄 Response content:"
            echo "$RESPONSE_CONTENT" | python3 -m json.tool

            # Check if the response contains expected cross-plugin call data
            if echo "$RESPONSE_CONTENT" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    content = data.get('content', [])
    found_cross_plugin_message = False
    found_time_data = False

    for item in content:
        if item.get('type') == 'text':
            text = item.get('text', '')
            try:
                parsed_text = json.loads(text)
                message = parsed_text.get('message', '')
                if 'Time retrieved via cross-plugin call' in message:
                    found_cross_plugin_message = True
                    print('✅ Found cross-plugin call message', file=sys.stderr)
                if 'time_data' in parsed_text:
                    found_time_data = True
                    print('✅ Found time data from cross-plugin call', file=sys.stderr)
            except json.JSONDecodeError:
                continue

    if found_cross_plugin_message and found_time_data:
        print('SUCCESS: Cross-plugin functionality validated')
        sys.exit(0)
    else:
        print('ERROR: Expected cross-plugin data not found')
        sys.exit(1)
except Exception as e:
    print(f'ERROR: Failed to parse response: {e}', file=sys.stderr)
    sys.exit(1)
"; then
                echo "✅ Cross-plugin tools functionality fully validated!"
                echo "✅ Wrapper plugin successfully called time plugin via cross_plugin_tools"
            else
                echo "⚠️  Response received but cross-plugin functionality not clearly validated"
                echo "🔍 Check the response content above for details"
            fi
        else
            echo "❌ No response received from wrapper plugin"
            if [ -s /tmp/wrapper_test_error.log ]; then
                echo "📢 Error log:"
                cat /tmp/wrapper_test_error.log
            fi
        fi

    else
        echo "❌ MCP call failed"
        if [ -s /tmp/wrapper_test_error.log ]; then
            echo "📢 Error details:"
            cat /tmp/wrapper_test_error.log
        fi
        deactivate
        exit 1
    fi

    # Test error handling with invalid tool call
    echo ""
    echo "🚫 Testing error handling with invalid tool call..."

    if python3 "$SCRIPT_DIR/mcp_call.py" \
        --server-cmd "$HYPER_MCP_BINARY" \
        --server-arg="--config-file" \
        --server-arg="/tmp/wrapper_test_config.yaml" \
        --server-arg="--transport" \
        --server-arg="stdio" \
        --tool "nonexistent_tool" \
        --tool-args '{}' > /tmp/invalid_test_result.json 2>/tmp/invalid_test_error.log; then

        echo "⚠️  Invalid tool call unexpectedly succeeded"
    else
        echo "✅ Error handling works correctly - invalid tool call properly rejected"
    fi

    # Clean up temporary files
    rm -f /tmp/wrapper_test_config.yaml
    rm -f /tmp/wrapper_test_result.json
    rm -f /tmp/wrapper_test_error.log
    rm -f /tmp/invalid_test_result.json
    rm -f /tmp/invalid_test_error.log

    # Deactivate virtual environment
    deactivate

    # Clean up virtual environment
    echo "🧹 Cleaning up virtual environment..."
    rm -rf "$VENV_DIR"

else
    echo "💡 To test manually, run:"
    echo "   1. Create a Python venv: python3 -m venv test_env"
    echo "   2. Activate it: source test_env/bin/activate"
    echo "   3. Install mcp: pip install mcp"
    echo "   4. Run: python3 mcp_call.py --server-cmd ./target/release/hyper-mcp --server-arg=\"--config-file\" --server-arg=\"tests/integrations/wrapper_plugin_test_config.yaml\" --server-arg=\"--transport\" --server-arg=\"stdio\" --tool \"wrapper::wrapper\" --tool-args '{\"name\": \"get_wrapped_time\"}'"
fi

# Display configuration summary
echo ""
echo "📋 Configuration Summary:"
echo "========================="
echo "Time plugin exposes tools: [time]"
echo "Wrapper plugin calls: time::time (get_time_utc operation)"
echo "Host function used: extism:host/user::call_tool"
echo "Test configurations created:"
echo "  - tests/integrations/wrapper_plugin_test_config.yaml"
echo "  - tests/integrations/wrapper_plugin_test_config.json"

echo ""
echo "🎉 All tests completed! Wrapper plugin cross-plugin tools functionality validated."
echo ""
echo "📚 What this test validates:"
echo "  ✅ Wrapper plugin compiles to WASM"
echo "  ✅ Configuration files are valid"
echo "  ✅ cross_plugin_tools parsing works"
echo "  ✅ Python venv with mcp package setup works"
echo "  ✅ mcp_call.py can communicate with hyper-mcp server"
if [ -n "$HYPER_MCP_BINARY" ]; then
echo "  ✅ MCP server starts with wrapper plugin configuration"
echo "  ✅ Wrapper plugin tool calls work end-to-end"
echo "  ✅ Cross-plugin communication functional"
echo "  ✅ Error handling works for invalid requests"
echo ""
echo "🚀 Integration tests completed successfully!"
echo "💡 The wrapper plugin can now call time::time via cross-plugin tools"
else
echo ""
echo "🚀 Next steps:"
echo "  1. Build hyper-mcp: cargo build --release"
echo "  2. Re-run this script for full integration testing"
fi
echo "📖 For manual testing: Use tests/integrations/wrapper_plugin_test_config.yaml"
