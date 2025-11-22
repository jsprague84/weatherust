# Weatherust Documentation

Welcome to the Weatherust documentation! This directory contains comprehensive documentation for users, developers, and contributors.

## 📚 Documentation Index

### For Users

Start here if you're using Weatherust:

- **[Main README](../README.md)** - Project overview, quick start, and basic usage
- **[CLI Commands](reference/CLI-COMMANDS.md)** - Command-line interface reference
- **[Webhook API](reference/WEBHOOK_API.md)** - HTTP webhook API documentation
- **[Bash Aliases](reference/BASH_ALIASES.md)** - Useful bash aliases for common operations

### For Developers

Essential reading for contributors and AI assistants:

- **[Claude Code Guide](development/CLAUDE_CODE_GUIDE.md)** - **START HERE** - Comprehensive development guide for AI-assisted development
- **[Contributing Guide](development/CONTRIBUTING.md)** - Development workflow, standards, and pull request process
- **[Modernization Summary](development/MODERNIZATION_SUMMARY.md)** - Recent improvements and migration guide
- **[Code Examples](development/MODERNIZATION_EXAMPLES.md)** - Practical examples of modern patterns

### Architecture & Design

System design and architecture documentation:

- **[Architecture Overview](architecture/ARCHITECTURE.md)** - System architecture, components, and data flow
- **[Cleanup Design](architecture/CLEANUP_DESIGN.md)** - Cleanup feature design and implementation

### Planning & Roadmap

Future plans and feature tracking:

- **[Features](planning/FEATURES.md)** - Feature tracking and planning
- **[2025 Improvements](planning/IMPROVEMENTS_2025.md)** - Planned improvements for 2025

### Service-Specific Documentation

Each service has its own README in its directory:

- **[healthmon](../healthmon/README.md)** - Docker container health monitoring
- **[updatectl](../updatectl/README.md)** - Update controller and cleanup tool
- **[updatemon](../updatemon/README.md)** - Update monitoring service

## 🗂️ Documentation Structure

```
docs/
├── README.md                    # This file - documentation index
├── development/                 # Developer documentation
│   ├── CLAUDE_CODE_GUIDE.md   # AI assistant development guide
│   ├── CONTRIBUTING.md        # Contributing workflow
│   ├── MODERNIZATION_SUMMARY.md
│   └── MODERNIZATION_EXAMPLES.md
├── architecture/                # System architecture
│   ├── ARCHITECTURE.md        # Main architecture doc
│   └── CLEANUP_DESIGN.md      # Feature designs
├── reference/                   # API and command reference
│   ├── CLI-COMMANDS.md        # CLI reference
│   ├── WEBHOOK_API.md         # Webhook API
│   └── BASH_ALIASES.md        # Helper scripts
├── planning/                    # Future plans
│   ├── FEATURES.md
│   └── IMPROVEMENTS_2025.md
└── archive/                     # Historical documentation
    └── README.old.md          # Previous README version
```

## 🎯 Quick Links by Role

### New Contributors
1. Read [Main README](../README.md)
2. Review [Contributing Guide](development/CONTRIBUTING.md)
3. Check [Architecture Overview](architecture/ARCHITECTURE.md)

### AI Assistants (Claude Code, etc.)
1. **Start with** [Claude Code Guide](development/CLAUDE_CODE_GUIDE.md)
2. Reference [Code Examples](development/MODERNIZATION_EXAMPLES.md)
3. Check [Architecture](architecture/ARCHITECTURE.md) for system design

### Users
1. [Main README](../README.md) - Installation and basic usage
2. [CLI Commands](reference/CLI-COMMANDS.md) - Command reference
3. Service READMEs - Service-specific documentation

### Operators/DevOps
1. [Main README](../README.md) - Deployment guide
2. [Webhook API](reference/WEBHOOK_API.md) - Automation integration
3. [Architecture](architecture/ARCHITECTURE.md) - System overview

## 📖 Documentation Standards

- All documentation uses **GitHub-flavored Markdown**
- Code examples include language tags for syntax highlighting
- External links use absolute URLs
- Internal links use relative paths from the file location
- Each major document includes a table of contents
- Keep documentation up-to-date with code changes

## 🔄 Keeping Documentation Updated

When making changes to the codebase:

1. Update relevant documentation in the same PR
2. Keep code examples accurate
3. Update version numbers and dates
4. Cross-check internal links
5. Review documentation checklist in PR template

## 📝 Documentation Conventions

- **Emojis**: Used sparingly for visual hierarchy in user-facing docs
- **Code Blocks**: Always include language identifier
- **File Paths**: Use absolute paths from repository root when referencing files
- **Line References**: Include line numbers when referencing specific code locations

## 🆘 Getting Help

- **Issues**: [GitHub Issues](https://github.com/jsprague84/weatherust/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jsprague84/weatherust/discussions)
- **Documentation**: Start with the [Claude Code Guide](development/CLAUDE_CODE_GUIDE.md)

---

**Last Updated**: 2025-01-22
**Documentation Version**: 2.0
