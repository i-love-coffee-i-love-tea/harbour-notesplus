//! LLM API Client supporting Ollama native (/api/chat) and OpenAI / Xiaomi MiMoCode (/v1/chat/completions).

use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::agent::tools::{get_available_tools, ToolCall, ToolDefinition};
use crate::constants::{DEFAULT_AI_ENDPOINT, DEFAULT_AI_MODEL, DEFAULT_AI_TIMEOUT_SECS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum LlmProvider {
    #[default]
    Ollama,
    OpenAiCompatible,
}

impl std::str::FromStr for LlmProvider {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" | "mimocode" | "compatible" => Ok(LlmProvider::OpenAiCompatible),
            "ollama" => Ok(LlmProvider::Ollama),
            _ => Ok(LlmProvider::Ollama),
        }
    }
}


/// Default endpoint URL for a local Ollama instance.
pub const DEFAULT_OLLAMA_ENDPOINT: &str = DEFAULT_AI_ENDPOINT;

/// Default model name when no specific model is configured.
pub const DEFAULT_MODEL: &str = DEFAULT_AI_MODEL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub endpoint_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    #[serde(default)]
    pub allow_self_signed: bool,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Ollama,
            endpoint_url: DEFAULT_OLLAMA_ENDPOINT.to_string(),
            model: DEFAULT_MODEL.to_string(),
            api_key: None,
            timeout_secs: DEFAULT_AI_TIMEOUT_SECS,
            allow_self_signed: false,
            system_prompt: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool_result(tool_call_id: Option<String>, name: String, content: String) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id,
            name: Some(name),
        }
    }
}

/// A model available on the configured LLM server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssistantResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Network connection error: {0}")]
    Network(String),
    #[error("HTTP error ({status}): {body}")]
    Http { status: u16, body: String },
    #[error("JSON serialization/deserialization error: {0}")]
    Json(String),
    #[error("Invalid LLM response: {0}")]
    InvalidResponse(String),
    #[error("Timeout waiting for LLM response")]
    Timeout,
}

pub struct LlmClient {
    config: LlmConfig,
}

#[derive(Debug)]
struct NoCertificateVerification(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Builds a rustls `ClientConfig` that accepts any (including self-signed or invalid) server certificate.
pub fn build_insecure_tls_client_config() -> rustls::ClientConfig {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let crypto_provider = Arc::new(rustls::crypto::ring::default_provider());
    rustls::ClientConfig::builder_with_provider(crypto_provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertificateVerification(crypto_provider)))
        .with_no_client_auth()
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Builds a configured ureq Agent honoring timeouts and self-signed certificate settings.
    pub fn build_agent(&self) -> ureq::Agent {
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let connect_timeout = Duration::from_secs(self.config.timeout_secs.clamp(10, 30));
        let mut builder = ureq::AgentBuilder::new()
            .timeout_connect(connect_timeout)
            .timeout_read(timeout)
            .timeout_write(Duration::from_secs(30))
            .timeout(timeout);

        if self.config.allow_self_signed {
            builder = builder.tls_config(Arc::new(build_insecure_tls_client_config()));
        }

        builder.build()
    }

    /// Resolves the actual chat endpoint URL based on provider type and configured base URL.
    pub fn resolve_chat_url(&self) -> String {
        let base = self.config.endpoint_url.trim().trim_end_matches('/');
        match self.config.provider {
            LlmProvider::Ollama => {
                if base.ends_with("/api/chat") || base.ends_with("/v1/chat/completions") {
                    base.to_string()
                } else if base.ends_with("/api") {
                    format!("{}/chat", base)
                } else {
                    format!("{}/api/chat", base)
                }
            }
            LlmProvider::OpenAiCompatible => {
                if base.ends_with("/v1/chat/completions") || base.ends_with("/api/chat") {
                    base.to_string()
                } else if base.ends_with("/v1") {
                    format!("{}/chat/completions", base)
                } else {
                    format!("{}/v1/chat/completions", base)
                }
            }
        }
    }

    /// Resolves the models listing endpoint URL based on provider type.
    pub fn resolve_models_url(&self) -> String {
        let base = self.config.endpoint_url.trim().trim_end_matches('/');
        match self.config.provider {
            LlmProvider::Ollama => {
                if base.ends_with("/api/tags") {
                    base.to_string()
                } else if base.ends_with("/api") {
                    format!("{}/tags", base)
                } else {
                    format!("{}/api/tags", base)
                }
            }
            LlmProvider::OpenAiCompatible => {
                if base.ends_with("/v1/models") {
                    base.to_string()
                } else if base.ends_with("/v1") {
                    format!("{}/models", base)
                } else {
                    format!("{}/v1/models", base)
                }
            }
        }
    }

    /// Fetches the list of available models from the configured LLM server.
    pub fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let url = self.resolve_models_url();
        let agent = self.build_agent();

        let mut req = agent.get(&url)
            .set("Content-Type", "application/json");

        if let Some(ref key) = self.config.api_key {
            if !key.trim().is_empty() {
                req = req.set("Authorization", &format!("Bearer {}", key.trim()));
            }
        }

        let resp = match req.call() {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_text = resp.into_string().unwrap_or_default();
                return Err(LlmError::Http { status: code, body: err_text });
            }
            Err(ureq::Error::Transport(t)) => {
                return Err(LlmError::Network(format!("Failed to fetch models from {}: {}", url, t)));
            }
        };

        let body: Value = resp.into_json()
            .map_err(|e| LlmError::Json(format!("Failed to parse models response: {}", e)))?;

        match self.config.provider {
            LlmProvider::Ollama => {
                // Ollama: { "models": [{ "name": "llama3.2:latest", ... }] }
                let models = body.get("models")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| LlmError::InvalidResponse("Missing 'models' array in Ollama response".to_string()))?;

                let mut result = Vec::new();
                for m in models {
                    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let display_name = name.split(':').next().unwrap_or(name).to_string();
                    let details = m.get("details");
                    let parameter_size = details.and_then(|d| d.get("parameter_size")).and_then(|v| v.as_str()).map(|s| s.to_string());
                    let quantization = details.and_then(|d| d.get("quantization_level")).and_then(|v| v.as_str()).map(|s| s.to_string());
                    result.push(ModelInfo {
                        id: name.to_string(),
                        name: display_name,
                        parameter_size,
                        quantization,
                    });
                }
                Ok(result)
            }
            LlmProvider::OpenAiCompatible => {
                // OpenAI: { "data": [{ "id": "gpt-4", ... }] }
                let data = body.get("data")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| LlmError::InvalidResponse("Missing 'data' array in OpenAI response".to_string()))?;

                let mut result = Vec::new();
                for m in data {
                    let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    result.push(ModelInfo {
                        id: id.to_string(),
                        name: id.to_string(),
                        parameter_size: None,
                        quantization: None,
                    });
                }
                Ok(result)
            }
        }
    }

    /// Formats the request payload for the appropriate provider with optional streaming flag.
    pub fn build_request_body(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Value {
        self.build_request_body_with_stream(messages, tools, false)
    }

    /// Formats the request payload for the appropriate provider with explicit stream parameter.
    pub fn build_request_body_with_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        stream: bool,
    ) -> Value {
        let serialized_messages: Vec<Value> = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| match self.config.provider {
                LlmProvider::Ollama => {
                    let mut obj = serde_json::Map::new();
                    obj.insert("role".to_string(), json!(msg.role));
                    obj.insert("content".to_string(), json!(msg.content.as_deref().unwrap_or("")));

                    if let Some(ref tool_calls) = msg.tool_calls {
                        let tc_vals: Vec<Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                let mut tc_obj = serde_json::Map::new();
                                if let Some(ref id) = tc.id {
                                    tc_obj.insert("id".to_string(), json!(id));
                                }
                                tc_obj.insert("type".to_string(), json!("function"));
                                tc_obj.insert(
                                    "function".to_string(),
                                    json!({
                                        "name": tc.function.name,
                                        "arguments": tc.function.arguments,
                                    }),
                                );
                                Value::Object(tc_obj)
                            })
                            .collect();
                        obj.insert("tool_calls".to_string(), json!(tc_vals));
                    }

                    if msg.role == "tool" {
                        if let Some(ref name) = msg.name {
                            obj.insert("tool_name".to_string(), json!(name));
                        }
                    }

                    Value::Object(obj)
                }
                LlmProvider::OpenAiCompatible => {
                    let mut obj = serde_json::Map::new();
                    obj.insert("role".to_string(), json!(msg.role));

                    if msg.role == "assistant" {
                        if let Some(ref tool_calls) = msg.tool_calls {
                            if let Some(ref c) = msg.content {
                                obj.insert("content".to_string(), json!(c));
                            } else {
                                obj.insert("content".to_string(), Value::Null);
                            }

                            let tc_vals: Vec<Value> = tool_calls
                                .iter()
                                .enumerate()
                                .map(|(tc_idx, tc)| {
                                    let call_id = tc
                                        .id
                                        .clone()
                                        .unwrap_or_else(|| format!("call_{}_{}", idx, tc_idx));
                                    let args_str = if tc.function.arguments.is_string() {
                                        tc.function.arguments.as_str().unwrap().to_string()
                                    } else {
                                        serde_json::to_string(&tc.function.arguments)
                                            .unwrap_or_else(|_| "{}".to_string())
                                    };
                                    json!({
                                        "id": call_id,
                                        "type": "function",
                                        "function": {
                                            "name": tc.function.name,
                                            "arguments": args_str,
                                        }
                                    })
                                })
                                .collect();
                            obj.insert("tool_calls".to_string(), json!(tc_vals));
                        } else {
                            obj.insert(
                                "content".to_string(),
                                json!(msg.content.as_deref().unwrap_or("")),
                            );
                        }
                    } else if msg.role == "tool" {
                        let call_id = msg
                            .tool_call_id
                            .clone()
                            .unwrap_or_else(|| format!("call_{}_0", idx.saturating_sub(1)));
                        obj.insert("tool_call_id".to_string(), json!(call_id));
                        obj.insert(
                            "content".to_string(),
                            json!(msg.content.as_deref().unwrap_or("")),
                        );
                    } else {
                        obj.insert(
                            "content".to_string(),
                            json!(msg.content.as_deref().unwrap_or("")),
                        );
                    }

                    Value::Object(obj)
                }
            })
            .collect();

        let mut body = json!({
            "model": self.config.model,
            "messages": serialized_messages,
            "stream": stream
        });
        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }
        body
    }

    /// Parses the raw JSON response from either Ollama or OpenAI/MiMoCode.
    pub fn parse_response(provider: LlmProvider, json_val: &Value) -> Result<AssistantResponse, LlmError> {
        match provider {
            LlmProvider::Ollama => {
                // Ollama format: { "message": { "role": "assistant", "content": "...", "tool_calls": [...] } }
                let message = json_val.get("message").ok_or_else(|| {
                    LlmError::InvalidResponse("Missing 'message' field in Ollama response".to_string())
                })?;

                let content = message.get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());

                let tool_calls = Self::extract_tool_calls(message.get("tool_calls"))?;

                Ok(AssistantResponse {
                    content,
                    tool_calls,
                })
            }
            LlmProvider::OpenAiCompatible => {
                // OpenAI format: { "choices": [ { "message": { "role": "assistant", "content": "...", "tool_calls": [...] } } ] }
                let choices = json_val.get("choices")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        LlmError::InvalidResponse("Missing or invalid 'choices' array in OpenAI response".to_string())
                    })?;

                if choices.is_empty() {
                    return Err(LlmError::InvalidResponse("Empty choices in response".to_string()));
                }

                let message = choices[0].get("message").ok_or_else(|| {
                    LlmError::InvalidResponse("Missing 'message' in first choice".to_string())
                })?;

                let content = message.get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());

                let tool_calls = Self::extract_tool_calls(message.get("tool_calls"))?;

                Ok(AssistantResponse {
                    content,
                    tool_calls,
                })
            }
        }
    }

    fn extract_tool_calls(val: Option<&Value>) -> Result<Vec<ToolCall>, LlmError> {
        let mut results = Vec::new();
        if let Some(calls_val) = val {
            if let Some(arr) = calls_val.as_array() {
                for item in arr {
                    let id = item.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let func_obj = item.get("function").ok_or_else(|| {
                        LlmError::InvalidResponse("Missing 'function' inside tool_call".to_string())
                    })?;

                    let name = func_obj.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                        LlmError::InvalidResponse("Missing 'name' inside function definition".to_string())
                    })?.to_string();

                    let raw_args = func_obj.get("arguments").cloned().unwrap_or(Value::Null);
                    let arguments = match raw_args {
                        Value::String(s) => {
                            serde_json::from_str(&s).unwrap_or_else(|_| json!({ "raw": s }))
                        }
                        Value::Object(_) => raw_args,
                        _ => json!({}),
                    };

                    results.push(ToolCall {
                        id,
                        tool_type: "function".to_string(),
                        function: crate::agent::tools::FunctionCall {
                            name,
                            arguments,
                        },
                    });
                }
            }
        }
        Ok(results)
    }

    /// Sends a chat completion request to the configured LLM endpoint.
    pub fn send_chat(&self, messages: &[ChatMessage]) -> Result<AssistantResponse, LlmError> {
        self.send_chat_streaming(messages, |_| {})
    }

    /// Sends a chat completion request to the configured LLM endpoint with token-by-token streaming callback.
    pub fn send_chat_streaming<F: FnMut(&str)>(
        &self,
        messages: &[ChatMessage],
        on_token: F,
    ) -> Result<AssistantResponse, LlmError> {
        let tools = get_available_tools();
        let body = self.build_request_body_with_stream(messages, &tools, true);
        let url = self.resolve_chat_url();

        let agent = self.build_agent();

        let mut req = agent.post(&url)
            .set("Content-Type", "application/json");

        if let Some(ref key) = self.config.api_key {
            if !key.trim().is_empty() {
                req = req.set("Authorization", &format!("Bearer {}", key.trim()));
            }
        }

        let resp = match req.send_json(body) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                let err_text = resp.into_string().unwrap_or_default();
                return Err(LlmError::Http {
                    status: code,
                    body: err_text,
                });
            }
            Err(ureq::Error::Transport(t)) => {
                let err_msg = match t.kind() {
                    ureq::ErrorKind::ConnectionFailed => format!("Connection failed to {}: {}", url, t),
                    _ => format!("{}", t),
                };
                return Err(LlmError::Network(err_msg));
            }
        };

        let reader = BufReader::new(resp.into_reader());

        match self.config.provider {
            LlmProvider::Ollama => Self::parse_ollama_stream(reader, on_token),
            LlmProvider::OpenAiCompatible => Self::parse_openai_stream(reader, on_token),
        }
    }

    /// Parses an Ollama NDJSON stream line-by-line.
    pub fn parse_ollama_stream<R: BufRead, F: FnMut(&str)>(
        reader: R,
        mut on_token: F,
    ) -> Result<AssistantResponse, LlmError> {
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls = Vec::new();

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| LlmError::Network(format!("Stream read error: {}", e)))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let json_val: Value = serde_json::from_str(trimmed)
                .map_err(|e| LlmError::Json(format!("Invalid JSON line {}: {}", e, trimmed)))?;

            if let Some(err_msg) = json_val.get("error").and_then(|v| v.as_str()) {
                return Err(LlmError::InvalidResponse(err_msg.to_string()));
            }

            if let Some(msg_obj) = json_val.get("message") {
                if let Some(token) = msg_obj.get("content").and_then(|v| v.as_str()) {
                    if !token.is_empty() {
                        accumulated_content.push_str(token);
                        on_token(token);
                    }
                }

                if let Some(tcs) = msg_obj.get("tool_calls") {
                    if let Ok(calls) = Self::extract_tool_calls(Some(tcs)) {
                        for call in calls {
                            accumulated_tool_calls.push(call);
                        }
                    }
                }
            }

            let is_done = json_val.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_done {
                break;
            }
        }

        Ok(AssistantResponse {
            content: if accumulated_content.is_empty() { None } else { Some(accumulated_content) },
            tool_calls: accumulated_tool_calls,
        })
    }

    /// Parses an OpenAI/MiMoCode SSE stream line-by-line.
    pub fn parse_openai_stream<R: BufRead, F: FnMut(&str)>(
        reader: R,
        mut on_token: F,
    ) -> Result<AssistantResponse, LlmError> {
        let mut accumulated_content = String::new();
        let mut tool_calls_by_index: std::collections::BTreeMap<usize, (Option<String>, String, String)> = std::collections::BTreeMap::new();

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| LlmError::Network(format!("Stream read error: {}", e)))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let payload = if let Some(stripped) = trimmed.strip_prefix("data:") {
                stripped.trim()
            } else {
                trimmed
            };

            if payload == "[DONE]" {
                break;
            }

            let json_val: Value = match serde_json::from_str(payload) {
                Ok(v) => v,
                Err(_) => continue, // Skip comments or non-JSON payloads
            };

            if let Some(err_obj) = json_val.get("error") {
                let msg = err_obj.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown OpenAI error");
                return Err(LlmError::InvalidResponse(msg.to_string()));
            }

            if let Some(choices) = json_val.get("choices").and_then(|v| v.as_array()) {
                if let Some(choice) = choices.first() {
                    if let Some(delta) = choice.get("delta") {
                        if let Some(token) = delta.get("content").and_then(|v| v.as_str()) {
                            if !token.is_empty() {
                                accumulated_content.push_str(token);
                                on_token(token);
                            }
                        }

                        if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                            for tc in tcs {
                                let idx = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                let entry = tool_calls_by_index.entry(idx).or_insert((None, String::new(), String::new()));
                                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                    entry.0 = Some(id.to_string());
                                }
                                if let Some(func) = tc.get("function") {
                                    if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                        entry.1.push_str(name);
                                    }
                                    if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                        entry.2.push_str(args);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut accumulated_tool_calls = Vec::new();
        for (_idx, (id, name, args_str)) in tool_calls_by_index {
            if !name.is_empty() {
                let args = serde_json::from_str::<Value>(&args_str).unwrap_or_else(|_| {
                    json!({ "raw": args_str })
                });
                accumulated_tool_calls.push(ToolCall {
                    id,
                    tool_type: "function".to_string(),
                    function: crate::agent::tools::FunctionCall {
                        name,
                        arguments: args,
                    },
                });
            }
        }

        Ok(AssistantResponse {
            content: if accumulated_content.is_empty() { None } else { Some(accumulated_content) },
            tool_calls: accumulated_tool_calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_chat_urls() {
        let mut cfg = LlmConfig {
            provider: LlmProvider::Ollama,
            endpoint_url: "http://192.168.1.100:11434".to_string(),
            model: "llama3.2".to_string(),
            api_key: None,
            timeout_secs: 30,
            allow_self_signed: false,
            system_prompt: None,
        };
        let client = LlmClient::new(cfg.clone());
        assert_eq!(client.resolve_chat_url(), "http://192.168.1.100:11434/api/chat");

        cfg.provider = LlmProvider::OpenAiCompatible;
        cfg.endpoint_url = "https://api.mimocode.com".to_string();
        let client_openai = LlmClient::new(cfg);
        assert_eq!(client_openai.resolve_chat_url(), "https://api.mimocode.com/v1/chat/completions");
    }

    #[test]
    fn test_parse_ollama_response_with_tool_call() {
        let json_data = json!({
            "model": "llama3.2",
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "function": {
                            "name": "edit_note",
                            "arguments": {
                                "filename": "todo.adoc",
                                "content": "= Todo\n* [x] Finish task",
                                "reason": "Mark task completed"
                            }
                        }
                    }
                ]
            }
        });

        let resp = LlmClient::parse_response(LlmProvider::Ollama, &json_data).unwrap();
        assert!(resp.content.is_none());
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].function.name, "edit_note");
        assert_eq!(resp.tool_calls[0].function.arguments["filename"], "todo.adoc");
    }

    #[test]
    fn test_parse_openai_response_with_string_arguments() {
        let json_data = json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "I am updating the note.",
                        "tool_calls": [
                            {
                                "id": "call_abc",
                                "type": "function",
                                "function": {
                                    "name": "create_note",
                                    "arguments": "{\"title\":\"Summary\",\"content\":\"= Summary\\nKey items\"}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let resp = LlmClient::parse_response(LlmProvider::OpenAiCompatible, &json_data).unwrap();
        assert_eq!(resp.content.as_deref(), Some("I am updating the note."));
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].id.as_deref(), Some("call_abc"));
        assert_eq!(resp.tool_calls[0].function.name, "create_note");
        assert_eq!(resp.tool_calls[0].function.arguments["title"], "Summary");
    }

    #[test]
    fn test_llm_config_timeout() {
        let cfg = LlmConfig {
            provider: LlmProvider::Ollama,
            endpoint_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            api_key: None,
            timeout_secs: 120,
            allow_self_signed: false,
            system_prompt: None,
        };
        let client = LlmClient::new(cfg);
        assert_eq!(client.config().timeout_secs, 120);
    }

    #[test]
    fn test_build_request_body_ollama_multi_turn() {
        let cfg = LlmConfig {
            provider: LlmProvider::Ollama,
            endpoint_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            api_key: None,
            timeout_secs: 60,
            allow_self_signed: false,
            system_prompt: None,
        };
        let client = LlmClient::new(cfg);

        let messages = vec![
            ChatMessage::system("You are an assistant."),
            ChatMessage::user("List all notes"),
            ChatMessage::assistant_tool_calls(vec![ToolCall {
                id: None,
                tool_type: "function".to_string(),
                function: crate::agent::tools::FunctionCall {
                    name: "list_notes".to_string(),
                    arguments: json!({}),
                },
            }]),
            ChatMessage::tool_result(None, "list_notes".to_string(), "[{\"filename\":\"a.adoc\"}]".to_string()),
            ChatMessage::assistant("Here is your note: a.adoc"),
            ChatMessage::user("What else do you know?"),
        ];

        let body = client.build_request_body(&messages, &[]);
        let msgs = body["messages"].as_array().expect("Must have messages array");
        assert_eq!(msgs.len(), 6);

        // System
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are an assistant.");

        // User 1
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "List all notes");

        // Assistant tool call
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], ""); // String, not null or missing!
        assert!(msgs[2]["tool_calls"].is_array());
        let tc = &msgs[2]["tool_calls"][0];
        assert_eq!(tc["function"]["name"], "list_notes");
        assert!(tc.get("id").is_none()); // No "id": null!

        // Tool result
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_name"], "list_notes");
        assert_eq!(msgs[3]["content"], "[{\"filename\":\"a.adoc\"}]");

        // Assistant response
        assert_eq!(msgs[4]["role"], "assistant");
        assert_eq!(msgs[4]["content"], "Here is your note: a.adoc");

        // User 2
        assert_eq!(msgs[5]["role"], "user");
        assert_eq!(msgs[5]["content"], "What else do you know?");
    }

    #[test]
    fn test_build_request_body_openai_multi_turn() {
        let cfg = LlmConfig {
            provider: LlmProvider::OpenAiCompatible,
            endpoint_url: "https://api.mimocode.com".to_string(),
            model: "mimo-code".to_string(),
            api_key: Some("secret".to_string()),
            timeout_secs: 60,
            allow_self_signed: false,
            system_prompt: None,
        };
        let client = LlmClient::new(cfg);

        let messages = vec![
            ChatMessage::system("You are an assistant."),
            ChatMessage::user("List all notes"),
            ChatMessage::assistant_tool_calls(vec![ToolCall {
                id: Some("call_abc123".to_string()),
                tool_type: "function".to_string(),
                function: crate::agent::tools::FunctionCall {
                    name: "list_notes".to_string(),
                    arguments: json!({}),
                },
            }]),
            ChatMessage::tool_result(Some("call_abc123".to_string()), "list_notes".to_string(), "[{\"filename\":\"a.adoc\"}]".to_string()),
            ChatMessage::assistant("Here is your note: a.adoc"),
            ChatMessage::user("What else do you know?"),
        ];

        let body = client.build_request_body(&messages, &[]);
        let msgs = body["messages"].as_array().expect("Must have messages array");
        assert_eq!(msgs.len(), 6);

        // Assistant tool call
        assert_eq!(msgs[2]["role"], "assistant");
        let tc = &msgs[2]["tool_calls"][0];
        assert_eq!(tc["id"], "call_abc123");
        assert_eq!(tc["function"]["name"], "list_notes");
        // Arguments must be stringified JSON in OpenAI
        assert_eq!(tc["function"]["arguments"], "{}");

        // Tool result
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_call_id"], "call_abc123");
        assert_eq!(msgs[3]["content"], "[{\"filename\":\"a.adoc\"}]");
    }

    #[test]
    fn test_parse_ollama_stream_tokens() {
        let stream_data = "\
{\"model\":\"llama3.2\",\"created_at\":\"2026-09-05T20:00:00Z\",\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"done\":false}\n\
{\"model\":\"llama3.2\",\"created_at\":\"2026-09-05T20:00:01Z\",\"message\":{\"role\":\"assistant\",\"content\":\" world\"},\"done\":false}\n\
{\"model\":\"llama3.2\",\"created_at\":\"2026-09-05T20:00:02Z\",\"message\":{\"role\":\"assistant\",\"content\":\"!\"},\"done\":true}\n";

        let mut tokens_received = Vec::new();
        let resp = LlmClient::parse_ollama_stream(stream_data.as_bytes(), |tok| {
            tokens_received.push(tok.to_string());
        }).expect("Stream parse should succeed");

        assert_eq!(tokens_received, vec!["Hello", " world", "!"]);
        assert_eq!(resp.content.as_deref(), Some("Hello world!"));
        assert!(resp.tool_calls.is_empty());
    }

    #[test]
    fn test_parse_ollama_stream_with_tool_calls() {
        let stream_data = "\
{\"model\":\"llama3.2\",\"created_at\":\"2026-09-05T20:00:00Z\",\"message\":{\"role\":\"assistant\",\"content\":\"\",\"tool_calls\":[{\"function\":{\"name\":\"list_notes\",\"arguments\":{}}}]},\"done\":true}\n";

        let mut tokens_received = Vec::new();
        let resp = LlmClient::parse_ollama_stream(stream_data.as_bytes(), |tok| {
            tokens_received.push(tok.to_string());
        }).expect("Stream parse should succeed");

        assert!(tokens_received.is_empty());
        assert!(resp.content.is_none());
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].function.name, "list_notes");
    }

    #[test]
    fn test_parse_openai_stream_tokens() {
        let stream_data = "\
data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"Async\"}}]}\n\
\n\
data: {\"choices\":[{\"delta\":{\"content\":\" stream\"}}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\" working\"}}]}\n\
data: [DONE]\n";

        let mut tokens_received = Vec::new();
        let resp = LlmClient::parse_openai_stream(stream_data.as_bytes(), |tok| {
            tokens_received.push(tok.to_string());
        }).expect("Stream parse should succeed");

        assert_eq!(tokens_received, vec!["Async", " stream", " working"]);
        assert_eq!(resp.content.as_deref(), Some("Async stream working"));
        assert!(resp.tool_calls.is_empty());
    }

    #[test]
    fn test_parse_openai_stream_tool_calls() {
        let stream_data = "\
data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"search_\"}}]}}]}\n\
data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"notes\",\"arguments\":\"{\\\"query\\\":\\\"test\\\"}\"}}]}}]}\n\
data: [DONE]\n";

        let resp = LlmClient::parse_openai_stream(stream_data.as_bytes(), |_| {})
            .expect("Stream parse should succeed");

        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(resp.tool_calls[0].function.name, "search_notes");
        assert_eq!(resp.tool_calls[0].function.arguments["query"], "test");
    }

    #[test]
    fn test_insecure_tls_client_config() {
        let cfg = LlmConfig {
            allow_self_signed: true,
            endpoint_url: "https://192.168.1.50:11434".to_string(),
            ..Default::default()
        };

        let client = LlmClient::new(cfg);
        assert!(client.config().allow_self_signed);
        let _agent = client.build_agent();

        let tls_cfg = build_insecure_tls_client_config();
        assert!(tls_cfg.alpn_protocols.is_empty());
    }
}
