# AI Assistance

> Source: [https://retejs.org/docs/development/ai-assistance](https://retejs.org/docs/development/ai-assistance)

# AI Assistance

Enhance your Rete.js development workflow with AI-powered coding assistants. This section covers different approaches to integrate AI tools with Rete.js documentation and development patterns.

## Quick Start

The fastest way to get AI assistance for Rete.js development is to use your AI-powered IDE's agent or command execution feature (available in Cursor, GitHub Copilot Chat, Windsurf, and similar tools). Simply ask your AI assistant:

```
promptRun npx rete-kit ai for [your intention]

```

Replace `[your intention]` with your actual goal, such as "learning Rete.js", "creating a new app", or "adding to my existing project". The AI agent will automatically determine the right tool and context options based on your description. This generates context-aware instructions for your IDE's AI assistant.

For manual usage or more details, see the Rete Kit AI documentation.

## Available Options

### Rete Kit AI

**Best for:** Automated, context-aware IDE integration

The `npx rete-kit ai` command (or `rete-kit ai` if installed globally) generates IDE-specific instruction files that provide contextually appropriate help based on your current situation:

- **Learning Rete.js** → `onboard` context
- **Creating new apps** → `boot` context
- **Working with existing code** → `dev` context
- **Plugin development** → `plugin` context

Check out the Rete Kit AI guide for complete documentation.

### LLMs.txt Integration

**Best for:** Manual integration with any AI tool

LLMs.txt provides Rete.js documentation in a format optimized for AI consumption. You can reference these URLs in your AI conversations or configuration files:

- **llms.txt** - Essential concepts and quick reference
- **llms-full.txt** - Complete documentation

See the LLMs.txt guide for integration examples.

## Which Should I Use?

- **Use Rete Kit AI** if you want automated setup with context-aware instructions for your IDE
- **Use LLMs.txt** if you prefer manual integration or need to reference documentation in AI conversations

Both approaches can be used together for comprehensive AI assistance.

## Related Documentation

- Rete Kit - Other Rete Kit features
- Getting Started - Quick start with AI assistance
