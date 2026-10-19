# Design

Packages:

- `Platform.Ai.Contracts`: requests, results, structured output, streaming, embeddings, usage,
  cost, provider status, and failure categories.
- `Platform.Ai`: routing, feature policy, timeout/retry hooks, redaction, quota, and telemetry.
- `Platform.Ai.OpenAiCompatible`, `Platform.Ai.Anthropic`, and `Platform.Ai.Ollama`: adapters.
- `Platform.Ai.Testing`: deterministic providers and usage recorders.

Every request carries an application feature key. Prompts and schemas remain application-owned.
OpenAI and DeepSeek may share an adapter only where their protocol behavior is demonstrably
compatible; Anthropic-specific features remain explicit.

The default development provider returns deterministic structured output or a documented
unavailable result. Production configuration must fail clearly when a selected provider is not
configured.
