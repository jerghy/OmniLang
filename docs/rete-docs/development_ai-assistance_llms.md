# LLMs.txt

> Source: [https://retejs.org/docs/development/ai-assistance/llms](https://retejs.org/docs/development/ai-assistance/llms)

# LLMs.txt

Enhance your visual programming experience with AI tools that understand Rete.js node editor patterns, dataflow concepts, and plugin architecture.

## What is LLMs.txt?

Building node editors and visual programming interfaces involves unique patterns and concepts that differ from traditional application development. Our AI integration through [LLMs.txt](https://llmstxt.org/) helps coding assistants understand:

- **Node-based architecture** and connection patterns
- **Dataflow vs Control flow** processing paradigms
- **Plugin composition** for extensible editors
- **Multi-framework rendering** (React, Vue, Angular, Svelte)
- **Graph processing** and manipulation techniques

This enables AI tools to provide contextually accurate suggestions for your visual programming projects.

## Resources

Our documentation is optimized for AI consumption through specialized endpoints that understand visual programming workflows:

- **llms.txt** - Essential node editor concepts, core APIs, and quick reference patterns
- **llms-full.txt** - Complete visual programming knowledge base including advanced patterns, plugin architecture, and processing engines

These endpoints are continuously updated to reflect the latest in node editor development practices and framework capabilities.

## How to Use

You can integrate Rete.js documentation with any AI coding assistant by providing the LLMs.txt URLs as context. This gives the AI comprehensive knowledge about node editor development patterns and best practices.

Rete Kit AI automates this integration process by generating IDE-specific instruction files that reference LLMs.txt. If you're using Cursor, GitHub Copilot, Windsurf, or similar tools, consider using `npx rete-kit ai` for context-aware, automated setup.

### Automated Setup with Rete Kit AI

For automated IDE integration, use Rete Kit AI which generates context-aware instructions that reference LLMs.txt:

```
bashnpx rete-kit ai --tool cursor --context onboard

```

This creates IDE-specific instruction files that automatically include references to LLMs.txt endpoints. See the Rete Kit AI documentation for details.

### Manual Integration

You can manually integrate LLMs.txt URLs with any AI tool by referencing them in your conversations or configuration files. This approach gives you full control over how and when the documentation is used.

#### Basic integration steps

1. Copy the LLMs.txt URL: `https://retejs.org/llms.txt` (or `llms-full.txt` for complete docs)
2. Reference it in your AI tool's context or conversation
3. Ask questions about Rete.js development, node editor patterns, or get code suggestions

### VS Code GitHub Copilot

Add the LLMs.txt URL to your workspace for enhanced context-aware suggestions:

1. Create a `.copilot-instructions.md` file in your project root
2. Add the reference:markdown`# Copilot Instructions

For Rete.js development, reference the documentation at:
https://retejs.org/llms-full.txt

Focus on node editor patterns, dataflow programming, and plugin architecture.
`
3. Copilot will now understand Rete.js concepts when providing code completions and suggestions

### Google Gemini Chat

Reference the documentation directly in your conversation:

```
promptI'm building a node editor with Rete.js. Please review the documentation at https://retejs.org/llms-full.txt and help me create a custom node that processes image data through multiple transformation steps.

```

### Model Context Protocol (MCP)

MCP-compatible tools (such as Context7, Claude Desktop, and other MCP-enabled applications) can fetch the Rete.js documentation as context for AI models:

1. Configure your MCP tool to use the Rete.js LLMs.txt endpoints as resources
2. Once configured, the complete Rete.js documentation is available as context for AI conversations
3. AI models have persistent access to node editor patterns, plugin architecture, and best practices

To set up MCP integration, add the Rete.js LLMs.txt URLs (`https://retejs.org/llms.txt` or `https://retejs.org/llms-full.txt`) as resources in your MCP-compatible tool's configuration. Check your MCP tool's documentation for specific setup instructions.

## Related Documentation

- Rete Kit AI - Automated IDE integration
- AI Assistance Overview - All AI integration options
