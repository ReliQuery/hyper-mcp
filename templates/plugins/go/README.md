# Go Plugin Template

A WebAssembly plugin template for building MCP (Model Context Protocol) plugins in Go using the hyper-mcp framework.

## Overview

This template provides a starter project for creating MCP plugins that run as WebAssembly modules. It includes all necessary dependencies and boilerplate code to implement MCP protocol handlers.

## Project Structure

```
.
├── cmd/
│   └── plugin/
│       └── main.go         # Main plugin implementation with WASM exports
├── pkg/
│   └── pdk/
│       └── types.go        # MCP protocol types
├── go.mod                  # Go module definition
├── go.sum                  # Go module checksums
├── Dockerfile              # Multi-stage build for compiling to WASM
└── .gitignore              # Git ignore rules
```

## Getting Started

### Prerequisites

- Go 1.22 or later
- Docker (for building WASM)
- `clang` and `lld` (for WASM compilation)

### Development

1. **Clone or use this template** to start your plugin project

2. **Implement plugin handlers** in `cmd/plugin/main.go`:

   > **Note:** You only need to implement the handlers relevant to your plugin. For example, if your plugin only provides tools, implement only `ListTools()` and `CallTool()`. All other handlers have default implementations that work out of the box.

   - `ListTools()` - Describe available tools
   - `CallTool()` - Execute a tool
   - `ListResources()` - List available resources
   - `ReadResource()` - Read resource contents
   - `ListPrompts()` - List available prompts
   - `GetPrompt()` - Get prompt details
   - `Complete()` - Provide auto-completion suggestions

3. **Build locally** (requires Docker for WASM target):
   ```sh
   docker build -t your-plugin-name .
   docker run --rm -v $(pwd):/workspace your-plugin-name cp /plugin.wasm /workspace/
   ```

### Dependencies

The template uses:

- **extism/go-pdk** - Plugin Development Kit for Extism
- Standard Go libraries for JSON serialization and time handling

## Plugin Handler Functions

Your plugin can implement any combination of the following handlers. **Only implement the handlers your plugin needs** - the template provides sensible defaults for everything else:

| Handler | Purpose | Required For |
|---------|---------|--------------|
| `ListTools()` | Declare available tools | Tool-providing plugins |
| `CallTool()` | Execute a tool | Tool-providing plugins |
| `ListResources()` | Declare available resources | Resource-providing plugins |
| `ListResourceTemplates()` | Declare resource templates | Dynamic resource plugins |
| `ReadResource()` | Read resource contents | Resource-providing plugins |
| `ListPrompts()` | Declare available prompts | Prompt-providing plugins |
| `GetPrompt()` | Retrieve a specific prompt | Prompt-providing plugins |
| `Complete()` | Provide auto-completions | Plugins supporting completions |
| `OnRootsListChanged()` | Handle root changes | Plugins reacting to root changes |

**Example: Tools-only plugin**

If your plugin only provides tools, you only need to implement:

```go
func ListTools(input pdk.ListToolsRequest) pdk.ListToolsResult {
    return pdk.ListToolsResult{
        Tools: []pdk.Tool{
            {
                Name: "greet",
                Description: ptrString("Greet a person"),
                InputSchema: pdk.ToolSchema{
                    Type: "object",
                    Properties: map[string]interface{}{
                        "name": map[string]interface{}{
                            "type": "string",
                            "description": "The person's name",
                        },
                    },
                    Required: []string{"name"},
                },
            },
        },
    }
}

func CallTool(input pdk.CallToolRequest) pdk.CallToolResult {
    switch input.Request.Name {
    case "greet":
        name, ok := input.Request.Arguments["name"].(string)
        if !ok {
            return pdk.CallToolResult{
                Content: []json.RawMessage{
                    []byte(`{"type":"text","text":"name argument required"}`),
                },
            }
        }
        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Hello, %s!"}`, name)),
            },
        }
    default:
        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Unknown tool: %s"}`, input.Request.Name)),
            },
        }
    }
}
```

All other handlers will use their default implementations.

## Host Functions

Your plugin can call these host functions to interact with the client and MCP server. Available through the `imports` package:

```go
import "plugin/pkg/pdk/imports"
```

### User Interaction

**`CreateElicitation(input ElicitRequestParamWithTimeout) (ElicitResult, error)`**

Request user input through the client's elicitation interface. Use this when your plugin needs user guidance, decisions, or confirmations during execution.

```go
result, err := imports.CreateElicitation(pdk.ElicitRequestParamWithTimeout{
    Message: "Please provide your name",
    RequestedSchema: pdk.Schema{
        Type: "object",
        Properties: map[string]json.RawMessage{
            "name": json.RawMessage(`{"type":"string"}`),
        },
    },
    Timeout: ptrInt64(30000), // 30 second timeout
})
```

### Message Generation

**`CreateMessage(input CreateMessageRequestParam) (CreateMessageResult, error)`**

Request message creation through the client's sampling interface. Use this when your plugin needs intelligent text generation or analysis with AI assistance.

```go
result, err := imports.CreateMessage(pdk.CreateMessageRequestParam{
    MaxTokens: 1024,
    Messages: []json.RawMessage{
        // conversation history
    },
    SystemPrompt: ptrString("You are a helpful assistant"),
})
```

### Resource Discovery

**`ListRoots() (ListRootsResult, error)`**

List the client's root directories or resources. Use this to discover what root resources (typically file system roots) are available and understand the scope of resources your plugin can access.

```go
roots, err := imports.ListRoots()
if err == nil {
    for _, root := range roots.Roots {
        fmt.Printf("Root: %s at %s\n", *root.Name, root.URI)
    }
}
```

### Logging

**`NotifyLoggingMessage(input LoggingMessageNotificationParam) error`**

Send diagnostic, informational, warning, or error messages to the client. The client's logging level determines which messages are processed and displayed.

```go
imports.NotifyLoggingMessage(pdk.LoggingMessageNotificationParam{
    Level: pdk.LoggingLevelInfo,
    Logger: ptrString("my_plugin"),
    Data: json.RawMessage(`{"message": "Processing started"}`),
})
```

### Progress Reporting

**`NotifyProgress(input ProgressNotificationParam) error`**

Report progress during long-running operations. Allows clients to display progress bars or status information to users.

```go
imports.NotifyProgress(pdk.ProgressNotificationParam{
    Progress: 50,
    ProgressToken: "task-1",
    Total: ptrFloat64(100),
})
```

### List Change Notifications

Notify the client when your plugin's available items change:

**`NotifyToolListChanged() error`**
- Call this when you add, remove, or modify available tools

**`NotifyResourceListChanged() error`**
- Call this when you add, remove, or modify available resources

**`NotifyPromptListChanged() error`**
- Call this when you add, remove, or modify available prompts

**`NotifyResourceUpdated(input ResourceUpdatedNotificationParam) error`**
- Call this when you modify the contents of a specific resource

```go
// When your plugin's tools change
imports.NotifyToolListChanged()

// When a specific resource is updated
imports.NotifyResourceUpdated(pdk.ResourceUpdatedNotificationParam{
    URI: "resource://my-resource",
})
```

### Example: Interactive Tool with Progress

```go
func CallTool(input pdk.CallToolRequest) pdk.CallToolResult {
    switch input.Request.Name {
    case "long_task":
        // Log start
        imports.NotifyLoggingMessage(pdk.LoggingMessageNotificationParam{
            Level: pdk.LoggingLevelInfo,
            Data: json.RawMessage(`{"message": "Starting long task"}`),
        })

        // Do work with progress updates
        for i := 0; i < 10; i++ {
            // ... do work ...
            imports.NotifyProgress(pdk.ProgressNotificationParam{
                Progress: float64((i + 1) * 10),
                ProgressToken: "task-1",
                Total: ptrFloat64(100),
            })
        }

        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(`{"type":"text","text":"Task completed"}`),
            },
        }
    default:
        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Unknown tool: %s"}`, input.Request.Name)),
            },
        }
    }
}
```

## Building for Distribution

### Using Docker

The included `Dockerfile` provides a multi-stage build that compiles your plugin to WebAssembly:

```sh
docker build -t your-registry/your-plugin-name .
docker run --rm -v $(pwd):/workspace your-registry/your-plugin-name cp /plugin.wasm /workspace/
```

The Docker build:
1. Compiles your Go code to `wasip1` target
2. Creates a minimal image containing only the compiled `plugin.wasm`
3. Outputs an OCI-compatible container image

### Manual Build

To build manually without Docker (requires Go 1.22+):

```sh
# Build for WASM
GOOS=wasip1 GOARCH=wasm CGO_ENABLED=0 go build -o plugin.wasm ./cmd/plugin

# Result is at: plugin.wasm
```

## Implementation Guide

### Creating a Tool

Here's an example of implementing a simple tool:

```go
func ListTools(input pdk.ListToolsRequest) pdk.ListToolsResult {
    return pdk.ListToolsResult{
        Tools: []pdk.Tool{
            {
                Name: "greet",
                Description: ptrString("Greet a person"),
                InputSchema: pdk.ToolSchema{
                    Type: "object",
                    Properties: map[string]interface{}{
                        "name": map[string]interface{}{
                            "type": "string",
                            "description": "The person's name",
                        },
                    },
                    Required: []string{"name"},
                },
            },
        },
    }
}

func CallTool(input pdk.CallToolRequest) pdk.CallToolResult {
    switch input.Request.Name {
    case "greet":
        name, ok := input.Request.Arguments["name"].(string)
        if !ok {
            return pdk.CallToolResult{
                Content: []json.RawMessage{
                    []byte(`{"type":"text","text":"name argument required"}`),
                },
            }
        }
        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Hello, %s!"}`, name)),
            },
        }
    default:
        return pdk.CallToolResult{
            Content: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Unknown tool: %s"}`, input.Request.Name)),
            },
        }
    }
}
```

### Creating a Resource

Example of implementing a resource:

```go
func ListResources(input pdk.ListResourcesRequest) pdk.ListResourcesResult {
    return pdk.ListResourcesResult{
        Resources: []pdk.Resource{
            {
                URI: "resource://example",
                Name: "Example Resource",
                Description: ptrString("An example resource"),
                MimeType: ptrString("text/plain"),
            },
        },
    }
}

func ReadResource(input pdk.ReadResourceRequest) pdk.ReadResourceResult {
    switch input.Request.URI {
    case "resource://example":
        return pdk.ReadResourceResult{
            Contents: []json.RawMessage{
                []byte(`{"uri":"resource://example","mimeType":"text/plain","text":"Resource content here"}`),
            },
        }
    default:
        return pdk.ReadResourceResult{
            Contents: []json.RawMessage{
                []byte(fmt.Sprintf(`{"type":"text","text":"Unknown resource: %s"}`, input.Request.URI)),
            },
        }
    }
}
```

## Helper Functions

The template includes some useful helper functions for working with pointers:

```go
// Helper to create string pointers
func ptrString(s string) *string {
    return &s
}

// Helper to create int64 pointers
func ptrInt64(i int64) *int64 {
    return &i
}

// Helper to create float64 pointers
func ptrFloat64(f float64) *float64 {
    return &f
}

// Helper to create bool pointers
func ptrBool(b bool) *bool {
    return &b
}
```

## Configuration in hyper-mcp

After building and publishing your plugin, configure it in hyper-mcp:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "oci://your-registry/your-plugin-name:latest"
    }
  }
}
```

For local development/testing:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "file:///path/to/plugin.wasm"
    }
  }
}
```

## Testing

To test your plugin locally:

1. Build it: `docker build -t my-plugin . && docker run --rm -v $(pwd):/workspace my-plugin cp /plugin.wasm /workspace/`
2. Update hyper-mcp's config to point to `file://` URL
3. Start hyper-mcp with `RUST_LOG=debug`
4. Test through Claude Desktop, Cursor IDE, or another MCP client

## Resources

- [hyper-mcp Documentation](https://github.com/tuananh/hyper-mcp)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Extism Go PDK](https://github.com/extism/go-pdk)
- [WebAssembly Documentation](https://webassembly.org/)
- [Example Plugins](https://github.com/tuananh/hyper-mcp/tree/main/examples/plugins)

## License

Same as hyper-mcp - Apache 2.0
