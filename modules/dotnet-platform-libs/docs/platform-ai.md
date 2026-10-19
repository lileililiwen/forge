# Platform AI

The AI packages expose provider-neutral contracts. Applications own feature keys, prompts,
structured-output schemas, model choices, retention decisions, and generated-content persistence.
Requests carry a feature key, optional model and timeout, cancellation, and non-sensitive
metadata. The default client records usage through an injected sink and emits only
feature/provider/model/latency/usage telemetry; prompt and response content is never written to
the default logger.

`Platform.Ai.Contracts` has no provider dependencies. `Platform.Ai` supplies a policy-gated
client and single- or feature-routed providers. A policy can reject quota, privacy, or data
handling violations before a provider is called. Providers report safe classified failures and
explicit capabilities, including unsupported structured output or embeddings.

`Platform.Ai.OpenAiCompatible` uses raw HTTP for OpenAI-compatible APIs. The same adapter can be
configured for DeepSeek with its base address and provider name; no SDK or provider plan mapping
is included. `Platform.Ai.Anthropic` preserves Anthropic request headers and message semantics.
`Platform.Ai.Ollama` targets a local `/api/chat` endpoint and does not require credentials.

Use `Platform.Ai.Testing` for deterministic development and test providers. `FakeAiProvider`
returns `fake:<feature-key>` and `RecordingUsageSink` captures only feature and usage values.
Production hosts must provide an explicitly configured provider and policy; missing cloud
credentials produce a safe authentication failure rather than exposing a secret or response.
