# AI Context

Files in this folder give an LLM enough context to continue development without
reading the entire codebase from scratch.

| File | What's in it |
|------|-------------|
| [architecture.md](architecture.md) | System design, startup sequence, session state, data files |
| [data-model.md](data-model.md) | DB schema, card numbering rules, auth.json format, role table |
| [commands.md](commands.md) | Every Tauri IPC command with parameter names and return types |
| [license-system.md](license-system.md) | Hardware-locked license + 14-day trial: algorithm, registry layout, UI flow |
| [workflows.md](workflows.md) | Build commands, branch map, how to add commands/screens, keygen recipe |

The full product specification is in [`../FEATURE_CONTRACT.md`](../FEATURE_CONTRACT.md).
