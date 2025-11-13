# Plugin Templates

This directory contains templates for creating plugins for hyper-mcp in various programming languages. Plugins extend hyper-mcp's functionality by providing tools, resources, prompts, and other MCP capabilities through WebAssembly modules.

## Available Templates

### Rust

The recommended language for building hyper-mcp plugins. Rust provides excellent performance, safety, and tooling for WebAssembly development.

- **Location**: `rust/`
- **Getting Started**: See [rust/README.md](./rust/README.md)
- **Use When**: You want the best performance, safety, and ecosystem support
- **Compile Target**: WebAssembly (`wasm32-wasip1`)

**Key Features:**
- Excellent WASM performance and code size
- Strong type system catches errors at compile time
- Rich ecosystem of crates for common tasks
- Memory-safe execution model
- Direct support for Extism PDK

## Quick Start

1. **Choose a template language** (currently Rust)
2. **Read the template README** for language-specific setup instructions
3. **Implement your plugin** by adding tools, resources, prompts, etc.
4. **Build and test locally** using the provided build instructions
5. **Publish to a registry** following the distribution guide

## Plugin Capabilities

Plugins can provide any combination of:

- **Tools** - Functions that clients can call with structured inputs
- **Resources** - URI-based references to files, data, or services
- **Resource Templates** - URI patterns for dynamic resource discovery
- **Prompts** - Pre-defined prompts for specific use cases
- **Completions** - Auto-completion suggestions for user input

## Plugin Development Workflow

```
1. Create project from template
   ↓
2. Implement plugin handlers
   ↓
3. Build to WebAssembly
   ↓
4. Test locally with hyper-mcp
   ↓
5. Build Docker image
   ↓
6. Push to registry (Docker Hub, GHCR, etc.)
   ↓
7. Configure in hyper-mcp's config.json
   ↓
8. Use in Claude Desktop, Cursor IDE, or other MCP clients
```

## Common Tasks

### Set Up Development Environment

Follow the language-specific template README (e.g., [rust/README.md](./rust/README.md)) for:
- Required tools and dependencies
- Target/runtime setup
- Local build instructions

### Implement Plugin Handlers

**Only implement what you need** - for example:
- Tools-only plugin: `list_tools()` + `call_tool()`
- Resources-only plugin: `list_resources()` + `read_resource()`
- Prompts-only plugin: `list_prompts()` + `get_prompt()`

See the template README for a complete handler reference table.

### Call Host Functions

Your plugin can call host functions to interact with the MCP client:
- Request user input with `create_elicitation()`
- Generate messages with `create_message()`
- Report progress with `notify_progress()`
- Send logs with `notify_logging_message()`
- Query available roots with `list_roots()`
- Notify about changes to tools, resources, or prompts

See the template README for complete host function documentation.

### Build for Production

Each template includes a `Dockerfile` for reproducible, multi-stage builds:

```bash
docker build -t your-registry/your-plugin-name .
docker push your-registry/your-plugin-name:latest
```

### Configure in hyper-mcp

Add your plugin to hyper-mcp's config file:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "oci://your-registry/your-plugin-name:latest"
    }
  }
}
```

For local development, use a file:// URL:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "file:///path/to/target/wasm32-wasip1/release/plugin.wasm"
    }
  }
}
```

## Resources

- [hyper-mcp Main README](https://github.com/tuananh/hyper-mcp#readme)
- [hyper-mcp Plugin Creation Guide](https://github.com/tuananh/hyper-mcp/blob/main/CREATING_PLUGINS.md)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Extism Documentation](https://docs.extism.org/)
- [Example Plugins](https://github.com/tuananh/hyper-mcp/tree/main/examples/plugins)

## Adding More Templates

To add a template for another language:

1. Create a new directory: `templates/plugins/your-language/`
2. Set up the build system for compiling to `wasm32-wasip1`
3. Create a `Dockerfile` for building OCI container images
4. Add a comprehensive `README.md` following the pattern from `rust/README.md`
5. Include example implementations of key handlers
6. Submit as a contribution to hyper-mcp

## Support

- Check the template README for language-specific questions
- See [CREATING_PLUGINS.md](../CREATING_PLUGINS.md) for general plugin development
- Review [example plugins](../examples/plugins/) for working implementations
- Open an issue on [GitHub](https://github.com/tuananh/hyper-mcp) for bugs or feature requests

Happy plugin building! 🚀
