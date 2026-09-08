# ADR-008: Extensible LLM Client Adapter for Local and Cloud AI Backends

#### Status
Accepted

#### Context
AI note assistants must accommodate diverse deployment models: users may run private, zero-data-leakage local LLMs via Ollama on their workstation/LAN, on-device models, or remote OpenAI-compatible endpoints (such as MiMoCode, OpenRouter, or vLLM).

Hardcoding provider-specific payload schemas or SSE stream parsing into the UI or agent session creates tight coupling and makes supporting new AI providers difficult.

#### Decision
Implement a modular, provider-agnostic `LlmClient` architecture in `notesplusplus-core`:
1. **Unified Request Abstraction**: Normalize model configuration (`provider`, `endpoint_url`, `model_name`, `api_key`, `timeout_secs`, `allow_self_signed`) across providers.
2. **Protocol Adapters**: Encapsulate request formatting and response translation inside provider-specific handlers:
   - **Ollama**: Translates prompt context, options, and NDJSON token streams (`response`, `done`).
   - **OpenAI-Compatible**: Formats `messages` arrays, system prompts, temperature parameters, and standard Server-Sent Events (SSE `data: {"choices":[{"delta":{"content":"..."}}]}`).
3. **Incremental Stream Pipeline**: Deliver unified token chunks asynchronously to both the native Sailfish UI bridge (`AgentBridge`) and the web server SSE endpoint (`/api/agent/chat`).

#### Consequences
##### Positive / Utility Delivered
- **Extensible Provider Support**: Adding or modifying LLM providers requires only self-contained adapter logic without destabilizing agent tools or UI layers.
- **Consistent Streaming Experience**: Real-time token streaming operates identically across mobile Silica views and desktop browser sessions.
- **Local & Cloud Flexibility**: Users seamlessly switch between local Ollama instances and cloud endpoints via Settings.

##### Trade-offs / Mitigations
- Provider-specific features (such as custom tool calling schemas or json mode flags) must be mapped into the normalized agent prompt interface.
