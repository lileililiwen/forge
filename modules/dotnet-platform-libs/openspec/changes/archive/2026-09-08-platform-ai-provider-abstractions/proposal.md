# Proposal: AI provider abstractions

## Why

Projects repeatedly implement OpenAI-compatible clients, Claude/Anthropic calls, deterministic
development stubs, retries, timeouts, structured output, usage tracking, and provider-specific
error handling.

## Scope

Add provider-neutral text generation, structured output, streaming, embeddings, usage/cost,
feature quotas, resilience, redaction, provider routing, and test contracts. Add optional
OpenAI-compatible, DeepSeek, Anthropic, and Ollama adapters behind separate packages.

## Non-goals

- no product prompts, domain schemas, or generated-content persistence;
- no claim that providers have identical semantics;
- no mandatory cloud account or API key;
- no automatic logging of user prompts or model responses.

## API impact

Adds public AI request/result, provider, routing, usage, policy, and testing contracts.
