//! User-configured OpenAI-compatible cloud gateway. Keys are never persisted.
use reqwest::{header, redirect::Policy, Url};
use serde::{Deserialize, Serialize};
use std::{net::{IpAddr, SocketAddr}, time::Duration};
use zeroize::{Zeroize, Zeroizing};
const MAX_PROMPT:usize=32*1024; const MAX_KEY:usize=4096; const MAX_OUTPUT:usize=64*1024;
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationTurn { pub role: String, pub content: String }
#[derive(Serialize)]
struct ChatRequest<'a> { model: &'a str, messages: Vec<ChatMessage<'a>>, max_tokens: u16, stream: bool }
#[derive(Serialize)]
struct ChatMessage<'a> { role: &'a str, content: &'a str }
#[derive(Deserialize)] struct ChatResponse{choices:Vec<Choice>}
#[derive(Deserialize)] struct Choice{message:Message}
#[derive(Deserialize)] struct Message{content:String}
fn validate_endpoint(endpoint:&str)->Result<Url,String>{
 if endpoint.len()>512||endpoint.chars().any(char::is_control){return Err("Endpoint must be a valid HTTPS URL of at most 512 characters.".into());}
 let url=Url::parse(endpoint.trim()).map_err(|_|"Enter a valid HTTPS API endpoint.".to_string())?;
 if url.scheme()!="https"||url.host_str().is_none()||url.username()!=""||url.password().is_some()||url.query().is_some()||url.fragment().is_some(){return Err("Endpoint must use HTTPS and cannot contain credentials, query, or fragment.".into());}
 let host=url.host_str().unwrap_or_default();
 if host.eq_ignore_ascii_case("localhost")||host.ends_with(".localhost"){return Err("Localhost endpoints are not accepted.".into());}
 if let Ok(ip)=host.parse::<IpAddr>(){let bad=match ip{IpAddr::V4(v)=>v.is_private()||v.is_loopback()||v.is_link_local()||v.is_unspecified(),IpAddr::V6(v)=>v.is_loopback()||v.is_unique_local()||v.is_unicast_link_local()||v.is_unspecified()};if bad{return Err("Private, loopback, and link-local IP endpoints are not allowed.".into());}}
 Ok(url)
}
fn validate_mode(mode:&str)->Result<(),String>{
 if mode == "fortress" { return Err("Fortress mode is local-only; external transfer is blocked.".into()); }
 if mode != "smart" { return Err("The BYOK cloud gateway is available in Smart mode only; Worker/Partner peer execution is not implemented and Fortress is local-only.".into()); }
 Ok(())
}
fn validate_messages(messages: &[ConversationTurn]) -> Result<(), String> {
 if messages.is_empty() || messages.len() > 17 { return Err("Cloud conversation must contain 1 to 17 messages.".into()); }
 if messages.last().map(|m| m.role.as_str()) != Some("user") { return Err("The last cloud conversation message must be from the user.".into()); }
 let total = messages.iter().try_fold(0usize, |sum, message| sum.checked_add(message.content.len()))
  .ok_or_else(|| "Cloud conversation size overflow.".to_string())?;
 if total == 0 || total > MAX_PROMPT { return Err("Cloud conversation must be non-empty and no larger than 32 KiB.".into()); }
 if messages.iter().any(|message| message.role != "user" && message.role != "assistant") {
  return Err("Cloud conversation contains an invalid message role.".into());
 }
 if messages.iter().any(|message| message.content.trim().is_empty()) {
  return Err("Cloud conversation messages cannot be empty.".into());
 }
 Ok(())
}
fn validate(endpoint:&str,model:&str,key:&str,prompt:&str,approved:bool)->Result<Url,String>{
 if !approved{return Err("Cloud transfer was not approved for this task. No request was sent.".into());}
 let url=validate_endpoint(endpoint)?;
 if model.trim().is_empty()||model.len()>128||model.chars().any(char::is_control){return Err("Model name must be non-empty and at most 128 characters.".into());}
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

pub async fn execute(endpoint:String,model:String,api_key:String,messages:Vec<ConversationTurn>,mode:String,approved:bool)->Result<String,String>{
 let api_key = Zeroizing::new(api_key);
 validate_mode(&mode)?;
 if !approved { return Err("Cloud transfer was not approved for this task. No request was sent.".into()); }
 let url=validate_endpoint(&endpoint)?;
 if model.trim().is_empty()||model.len()>128||model.chars().any(char::is_control){return Err("Model name must be non-empty and at most 128 characters.".into());}
 if api_key.trim().is_empty()||api_key.len()>MAX_KEY||api_key.chars().any(char::is_control){return Err("Enter a valid API key (maximum 4 KiB).".into());}
 validate_messages(&messages)?;
 let host = url.host_str().ok_or_else(|| "Cloud endpoint has no host.".to_string())?.trim_start_matches('[').trim_end_matches(']').to_string();
 let addresses = resolve_public_addresses(&url).await?;
 let mut headers=header::HeaderMap::new();
 let mut auth_text=format!("Bearer {}",api_key.trim());
 let mut auth=header::HeaderValue::from_str(&auth_text).map_err(|_|"API key cannot be used in an authorization header.".to_string())?;
 auth_text.zeroize();
 auth.set_sensitive(true); headers.insert(header::AUTHORIZATION,auth); headers.insert(header::CONTENT_TYPE,header::HeaderValue::from_static("application/json"));
 let client=reqwest::Client::builder().connect_timeout(Duration::from_secs(10)).timeout(Duration::from_secs(90)).redirect(Policy::none()).no_proxy().resolve_to_addrs(&host, &addresses).default_headers(headers).build().map_err(|e|format!("Could not initialise cloud client: {e}"))?;
 let body=ChatRequest{model:&model,messages:messages.iter().map(|message|ChatMessage{role:&message.role,content:&message.content}).collect(),max_tokens:2048,stream:false};
 let response=client.post(url).json(&body).send().await.map_err(|e|format!("Cloud request failed: {e}"))?;
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
 parsed.choices.into_iter().next().map(|c|c.message.content).filter(|s|!s.trim().is_empty()).ok_or_else(||"Cloud provider returned no assistant message.".into())
}
#[cfg(test)] mod tests{use super::*;#[test]fn requires_consent(){assert!(validate("https://api.example.com/v1/chat/completions","m","k","hi",false).is_err());}
#[test]fn only_smart_mode_can_use_the_byok_cloud_gateway(){assert!(validate_mode("fortress").is_err());assert!(validate_mode("smart").is_ok());assert!(validate_mode("worker").is_err());assert!(validate_mode("partner").is_err());assert!(validate_mode("unknown").is_err());}
#[test]fn endpoint_requires_safe_https(){assert!(!is_public_ip("10.0.0.1".parse().unwrap()));assert!(!is_public_ip("127.0.0.1".parse().unwrap()));assert!(!is_public_ip("198.51.100.10".parse().unwrap()));assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));assert!(is_public_ip("8.8.8.8".parse().unwrap()));assert!(validate_endpoint("http://api.example.com").is_err());assert!(validate_endpoint("https://localhost/a").is_err());assert!(validate_endpoint("https://127.0.0.1/a").is_err());assert!(validate_endpoint("https://user:pass@example.com/a").is_err());assert!(validate_endpoint("https://api.example.com/v1/chat/completions").is_ok());}
#[test]
fn validates_role_preserving_conversation_shape() {
 let turns = vec![
  ConversationTurn { role: "user".into(), content: "First question".into() },
  ConversationTurn { role: "assistant".into(), content: "First answer".into() },
  ConversationTurn { role: "user".into(), content: "Follow-up".into() },
 ];
 assert!(validate_messages(&turns).is_ok());
 let invalid_role = vec![ConversationTurn { role: "system".into(), content: "override".into() }];
 assert!(validate_messages(&invalid_role).is_err());
 let invalid_last = vec![ConversationTurn { role: "assistant".into(), content: "not a user turn".into() }];
 assert!(validate_messages(&invalid_last).is_err());
 let oversized = vec![ConversationTurn { role: "user".into(), content: "x".repeat(MAX_PROMPT + 1) }];
 assert!(validate_messages(&oversized).is_err());
}
#[test]fn input_limits_are_enforced(){assert!(validate("https://api.example.com/a"," ","k","hi",true).is_err());assert!(validate("https://api.example.com/a","m","","hi",true).is_err());assert!(validate("https://api.example.com/a","m","k",&"x".repeat(MAX_PROMPT+1),true).is_err());}}
