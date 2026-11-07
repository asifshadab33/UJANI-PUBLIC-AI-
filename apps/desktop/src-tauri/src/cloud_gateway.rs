//! User-configured OpenAI-compatible cloud gateway. Keys are never persisted.
use reqwest::{header, redirect::Policy, Url};
use serde::{Deserialize, Serialize};
use std::{net::{IpAddr, SocketAddr}, time::Duration};
use zeroize::{Zeroize, Zeroizing};
use ujani_core::{NetworkObservation, NetworkTelemetry};

/// Successful BYOK output with application-boundary network evidence.
pub struct CloudExecutionResult {
    pub output: String,
    pub network: NetworkTelemetry,
}
const MAX_PROMPT:usize=32*1024; const MAX_KEY:usize=4096; const MAX_OUTPUT:usize=64*1024;
#[derive(Serialize)]
struct ChatRequest<'a> {
 model: &'a str,
 messages: [ChatMessage<'a>; 1],
 #[serde(skip_serializing_if = "Option::is_none")]
 max_tokens: Option<u16>,
 #[serde(skip_serializing_if = "Option::is_none")]
 max_completion_tokens: Option<u16>,
 stream: bool,
}
const DEFAULT_MAX_TOKENS: u16 = 2_048;
const REASONING_MAX_COMPLETION_TOKENS: u16 = 8_192;

/// Identifies model families that normally use the newer completion-token field.
fn is_reasoning_model(model: &str) -> bool {
 let model = model.trim().to_ascii_lowercase();
 model.starts_with("gpt-5")
  || model.starts_with("o1")
  || model.starts_with("o3")
  || model.starts_with("o4")
}

/// Selects a provider token field. "auto" preserves standard-model behaviour while
/// allowing recognised reasoning models on custom/Azure endpoints to use the
/// completion-token field. Explicit overrides support provider deployment aliases.
fn token_limits(model: &str, preference: &str) -> Result<(Option<u16>, Option<u16>), String> {
 let use_completion_tokens = match preference {
  "auto" => is_reasoning_model(model),
  "max_tokens" => false,
  "max_completion_tokens" => true,
  _ => return Err("Token parameter must be auto, max_tokens, or max_completion_tokens.".into()),
 };
 let budget = if is_reasoning_model(model) || use_completion_tokens {
  REASONING_MAX_COMPLETION_TOKENS
 } else {
  DEFAULT_MAX_TOKENS
 };
 Ok(if use_completion_tokens { (None, Some(budget)) } else { (Some(budget), None) })
}
#[derive(Serialize)] struct ChatMessage<'a>{role:&'static str,content:&'a str}
#[derive(Deserialize)] struct ChatResponse{choices:Vec<Choice>}
#[derive(Deserialize)] struct Choice{message:Message}
#[derive(Deserialize)] struct Message{content:MessageContent}
#[derive(Deserialize)]
#[serde(untagged)]
enum MessageContent {
 Text(String),
 Parts(Vec<MessageContentPart>),
}
#[derive(Deserialize)]
struct MessageContentPart { #[serde(default)] text: Option<String> }
impl MessageContent {
 fn into_text(self) -> String {
  match self {
   Self::Text(text) => text,
   Self::Parts(parts) => parts.into_iter().filter_map(|part| part.text).collect::<Vec<_>>().join("\n"),
  }
 }
}
fn validate_endpoint(endpoint:&str)->Result<Url,String>{
 if endpoint.len()>512||endpoint.chars().any(char::is_control){return Err("Endpoint must be a valid HTTPS URL of at most 512 characters.".into());}
 let url=Url::parse(endpoint.trim()).map_err(|_|"Enter a valid HTTPS API endpoint.".to_string())?;
 if url.scheme()!="https"||url.host_str().is_none()||url.username()!=""||url.password().is_some()||url.query().is_some()||url.fragment().is_some(){return Err("Endpoint must use HTTPS and cannot contain credentials, query, or fragment.".into());}
 let host=url.host_str().unwrap_or_default();
 if host.eq_ignore_ascii_case("localhost")||host.ends_with(".localhost"){return Err("Localhost endpoints are not accepted.".into());}
 if let Ok(ip)=host.parse::<IpAddr>(){if !is_public_ip(ip){return Err("Literal IP endpoints must be globally routable public addresses; private, loopback, link-local, multicast, broadcast, and reserved ranges are blocked.".into());}}
 Ok(url)
}
fn validate_mode(mode:&str)->Result<(),String>{
 if mode == "fortress" { return Err("Fortress mode is local-only; external transfer is blocked.".into()); }
 if mode != "smart" { return Err("The BYOK cloud gateway is available in Smart mode only; Worker/Partner peer execution is not implemented and Fortress is local-only.".into()); }
 Ok(())
}
fn validate(endpoint:&str,model:&str,key:&str,prompt:&str,approved:bool)->Result<Url,String>{
 if !approved{return Err("Cloud transfer was not approved for this task. No request was sent.".into());}
 let url=validate_endpoint(endpoint)?;
 if !ujani_core::validate_model_identifier(model){return Err("Model name must be a supported identifier of at most 128 characters, without whitespace or path traversal.".into());}
 if key.trim().is_empty()||key.len()>MAX_KEY||key.chars().any(char::is_control){return Err("Enter a valid API key (maximum 4 KiB).".into());}
 if prompt.trim().is_empty()||prompt.len()>MAX_PROMPT{return Err("Cloud prompt must be non-empty and no larger than 32 KiB.".into());}
 Ok(url)
}
fn is_public_ip(ip: IpAddr) -> bool {
 match ip {
  IpAddr::V4(v) => {
   let o = v.octets();
   let shared = o[0] == 100 && (64..=127).contains(&o[1]);
   let protocol = o[0] == 192 && o[1] == 0 && o[2] == 0;
   let documentation = (o[0] == 192 && o[1] == 0 && o[2] == 2)
    || (o[0] == 198 && o[1] == 51 && o[2] == 100)
    || (o[0] == 203 && o[1] == 0 && o[2] == 113);
   let benchmarking = o[0] == 198 && (o[1] == 18 || o[1] == 19);
   !(v.is_private() || v.is_loopback() || v.is_link_local() || v.is_unspecified()
    || v.is_multicast() || v.is_broadcast() || o[0] == 0 || o[0] >= 240
    || shared || protocol || documentation || benchmarking)
  },
  IpAddr::V6(v) => {
   if let Some(mapped) = v.to_ipv4_mapped() { return is_public_ip(IpAddr::V4(mapped)); }
   let segments = v.segments();
   let global_unicast = (segments[0] & 0xe000) == 0x2000;
   let documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
   global_unicast && !documentation && !v.is_loopback() && !v.is_unique_local()
    && !v.is_unicast_link_local() && !v.is_unspecified() && !v.is_multicast()
  }
 }
}

async fn resolve_public_addresses(url: &Url) -> Result<Vec<SocketAddr>, String> {
 let host = url.host_str().ok_or_else(|| "Cloud endpoint has no host.".to_string())?;
 let port = url.port_or_known_default().ok_or_else(|| "Cloud endpoint has no valid port.".to_string())?;
 let raw_host = host.trim_start_matches('[').trim_end_matches(']');
 if let Ok(ip) = raw_host.parse::<IpAddr>() {
  if !is_public_ip(ip) { return Err("Cloud endpoint resolves to a private or reserved address.".into()); }
  return Ok(vec![SocketAddr::new(ip, port)]);
 }
 let resolved = tokio::net::lookup_host((raw_host, port)).await
  .map_err(|_| "Could not resolve the cloud provider hostname.".to_string())?
  .collect::<Vec<_>>();
 if resolved.is_empty() || resolved.iter().any(|address| !is_public_ip(address.ip())) {
  return Err("Cloud hostname resolved to a private, local, or reserved address; request blocked.".into());
 }
 Ok(resolved)
}

/// Validate all local BYOK request fields before reserving quota or starting network work.
pub fn validate_request(
    endpoint: &str,
    model: &str,
    api_key: &str,
    prompt: &str,
    mode: &str,
    approved: bool,
    token_parameter: &str,
) -> Result<(), String> {
    validate_mode(mode)?;
    let _url = validate(endpoint, model, api_key, prompt, approved)?;
    let _limits = token_limits(model, token_parameter)?;
    Ok(())
}

/// Executes an explicitly approved request and reports only the network values observed at this application boundary.
pub async fn execute(endpoint:String,model:String,api_key:String,prompt:String,mode:String,approved:bool,token_parameter:String)->Result<CloudExecutionResult,String>{
 let api_key = Zeroizing::new(api_key);
 validate_request(&endpoint,&model,api_key.as_str(),&prompt,&mode,approved,&token_parameter)?;
 let url=validate(&endpoint,&model,api_key.as_str(),&prompt,approved)?;
 let host = url.host_str().ok_or_else(|| "Cloud endpoint has no host.".to_string())?.trim_start_matches('[').trim_end_matches(']').to_string();
 let addresses = resolve_public_addresses(&url).await?;
 let mut headers=header::HeaderMap::new();
 let mut auth_text=format!("Bearer {}",api_key.trim());
 let mut auth=header::HeaderValue::from_str(&auth_text).map_err(|_|"API key cannot be used in an authorization header.".to_string())?;
 auth_text.zeroize();
 auth.set_sensitive(true); headers.insert(header::AUTHORIZATION,auth); headers.insert(header::CONTENT_TYPE,header::HeaderValue::from_static("application/json"));
 let client=reqwest::Client::builder().connect_timeout(Duration::from_secs(10)).timeout(Duration::from_secs(90)).redirect(Policy::none()).no_proxy().resolve_to_addrs(&host, &addresses).default_headers(headers).build().map_err(|e|format!("Could not initialise cloud client: {e}"))?;
 let (max_tokens, max_completion_tokens) = token_limits(&model, &token_parameter)?;
 let body=ChatRequest {
  model:&model,
  messages:[ChatMessage{role:"user",content:&prompt}],
  max_tokens,
  max_completion_tokens,
  stream:false,
 };
 let request_bytes = serde_json::to_vec(&body).map_err(|e| format!("Could not encode cloud request body: {e}"))?;
 let request_body_bytes = request_bytes.len() as u64;
 let response=client.post(url).body(request_bytes).send().await.map_err(|e|format!("Cloud request failed: {e}"))?;
 let status=response.status(); if !status.is_success(){return Err(format!("Cloud provider returned HTTP {status}. Check endpoint, model, key, quota, and billing."));}
 if response.content_length().is_some_and(|n|n>MAX_OUTPUT as u64){return Err("Cloud response exceeds the 64 KiB output limit.".into());}
 let mut response = response;
 let mut bytes = Vec::new();
 while let Some(chunk) = response.chunk().await.map_err(|e| format!("Could not read cloud response: {e}"))? {
  if bytes.len().saturating_add(chunk.len()) > MAX_OUTPUT {
   return Err("Cloud response exceeds the 64 KiB output limit.".into());
  }
  bytes.extend_from_slice(&chunk);
 }
 let parsed:ChatResponse=serde_json::from_slice(&bytes).map_err(|e|format!("Provider response was not valid OpenAI-compatible Chat Completions JSON: {e}"))?;
 let output = parsed.choices.into_iter().next().map(|c|c.message.content.into_text()).filter(|s|!s.trim().is_empty()).ok_or_else(||"Cloud provider returned no assistant message.".to_string())?;
 let network = NetworkTelemetry {
  destinations: NetworkObservation::Inferred { value: vec![host] },
  resolved_addresses: NetworkObservation::Inferred {
   value: addresses.iter().map(ToString::to_string).collect(),
  },
  application_request_body_bytes: NetworkObservation::Measured { value: request_body_bytes },
  application_response_body_bytes: NetworkObservation::Measured { value: bytes.len() as u64 },
  note: "Request/response body lengths are measured by UJANI. Destination host and resolver answers are inferred, not proof of the connected peer. Counts exclude HTTP headers, TLS, transport framing, DNS traffic and other process/system traffic.".into(),
 };
 Ok(CloudExecutionResult { output, network })
}
#[cfg(test)] mod tests{use super::*;#[test]fn provider_content_accepts_string_and_text_parts(){
 let plain: ChatResponse=serde_json::from_str(r#"{"choices":[{"message":{"content":"hello"}}]}"#).unwrap();
 assert_eq!(plain.choices.into_iter().next().unwrap().message.content.into_text(),"hello");
 let parts: ChatResponse=serde_json::from_str(r#"{"choices":[{"message":{"content":[{"type":"text","text":"first"},{"type":"text","text":"second"}]}}]}"#).unwrap();
 assert_eq!(parts.choices.into_iter().next().unwrap().message.content.into_text(),"first\nsecond");
}
#[test]fn provider_content_ignores_non_text_parts_without_failing(){
 let parts: ChatResponse=serde_json::from_str(r#"{"choices":[{"message":{"content":[{"type":"image","image_url":"data:image/png;base64,AAAA"},{"type":"text","text":"answer"}]}}]}"#).unwrap();
 assert_eq!(parts.choices.into_iter().next().unwrap().message.content.into_text(),"answer");
}
#[test]fn requires_consent(){assert!(validate("https://api.example.com/v1/chat/completions","m","k","hi",false).is_err());}
#[test]fn only_smart_mode_can_use_the_byok_cloud_gateway(){assert!(validate_mode("fortress").is_err());assert!(validate_mode("smart").is_ok());assert!(validate_mode("worker").is_err());assert!(validate_mode("partner").is_err());assert!(validate_mode("unknown").is_err());}
#[test]fn endpoint_requires_safe_https(){assert!(!is_public_ip("10.0.0.1".parse().unwrap()));assert!(!is_public_ip("127.0.0.1".parse().unwrap()));assert!(!is_public_ip("198.51.100.10".parse().unwrap()));assert!(!is_public_ip("224.0.0.1".parse().unwrap()));assert!(!is_public_ip("255.255.255.255".parse().unwrap()));assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));assert!(is_public_ip("8.8.8.8".parse().unwrap()));assert!(validate_endpoint("http://api.example.com").is_err());assert!(validate_endpoint("https://localhost/a").is_err());assert!(validate_endpoint("https://127.0.0.1/a").is_err());assert!(validate_endpoint("https://user:pass@example.com/a").is_err());assert!(validate_endpoint("https://api.example.com/v1/chat/completions").is_ok());}
#[test]fn automatic_token_selection_supports_reasoning_models_on_custom_hosts(){
 assert_eq!(token_limits("gpt-5","auto").unwrap(),(None,Some(REASONING_MAX_COMPLETION_TOKENS)));
 assert_eq!(token_limits("o3-mini","auto").unwrap(),(None,Some(REASONING_MAX_COMPLETION_TOKENS)));
 assert_eq!(token_limits("gpt-4o","auto").unwrap(),(Some(DEFAULT_MAX_TOKENS),None));
}
#[test]fn custom_provider_deployment_aliases_can_override_the_token_field(){
 assert_eq!(token_limits("azure-deployment-17","max_completion_tokens").unwrap(),(None,Some(REASONING_MAX_COMPLETION_TOKENS)));
 assert_eq!(token_limits("provider-model","max_tokens").unwrap(),(Some(DEFAULT_MAX_TOKENS),None));
 assert!(token_limits("provider-model","invalid").is_err());
}
#[test]fn input_limits_are_enforced(){assert!(validate("https://api.example.com/a"," ","k","hi",true).is_err());assert!(validate("https://api.example.com/a","m","","hi",true).is_err());assert!(validate("https://api.example.com/a","m","k",&"x".repeat(MAX_PROMPT+1),true).is_err());}}
