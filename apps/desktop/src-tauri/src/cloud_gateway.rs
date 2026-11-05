//! User-configured OpenAI-compatible cloud gateway. Keys are never persisted.
use reqwest::{header, redirect::Policy, Url};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Duration};
const MAX_PROMPT:usize=32*1024; const MAX_KEY:usize=4096; const MAX_OUTPUT:usize=64*1024;
#[derive(Serialize)] struct ChatRequest<'a>{model:&'a str,messages:[ChatMessage<'a>;1],max_tokens:u16,stream:bool}
#[derive(Serialize)] struct ChatMessage<'a>{role:&'static str,content:&'a str}
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
fn validate(endpoint:&str,model:&str,key:&str,prompt:&str,approved:bool)->Result<Url,String>{
 if !approved{return Err("Cloud transfer was not approved for this task. No request was sent.".into());}
 let url=validate_endpoint(endpoint)?;
 if model.trim().is_empty()||model.len()>128||model.chars().any(char::is_control){return Err("Model name must be non-empty and at most 128 characters.".into());}
 if key.trim().is_empty()||key.len()>MAX_KEY||key.chars().any(char::is_control){return Err("Enter a valid API key (maximum 4 KiB).".into());}
 if prompt.trim().is_empty()||prompt.len()>MAX_PROMPT{return Err("Cloud prompt must be non-empty and no larger than 32 KiB.".into());}
 Ok(url)
}
pub async fn execute(endpoint:String,model:String,api_key:String,prompt:String,approved:bool)->Result<String,String>{
 let url=validate(&endpoint,&model,&api_key,&prompt,approved)?;
 let mut headers=header::HeaderMap::new();
 let mut auth=header::HeaderValue::from_str(&format!("Bearer {}",api_key.trim())).map_err(|_|"API key cannot be used in an authorization header.".to_string())?;
 auth.set_sensitive(true); headers.insert(header::AUTHORIZATION,auth); headers.insert(header::CONTENT_TYPE,header::HeaderValue::from_static("application/json"));
 let client=reqwest::Client::builder().connect_timeout(Duration::from_secs(10)).timeout(Duration::from_secs(90)).redirect(Policy::none()).default_headers(headers).build().map_err(|e|format!("Could not initialise cloud client: {e}"))?;
 let body=ChatRequest{model:&model,messages:[ChatMessage{role:"user",content:&prompt}],max_tokens:2048,stream:false};
 let response=client.post(url).json(&body).send().await.map_err(|e|format!("Cloud request failed: {e}"))?;
 let status=response.status(); if !status.is_success(){return Err(format!("Cloud provider returned HTTP {status}. Check endpoint, model, key, quota, and billing."));}
 if response.content_length().is_some_and(|n|n>MAX_OUTPUT as u64){return Err("Cloud response exceeds the 64 KiB output limit.".into());}
 let bytes=response.bytes().await.map_err(|e|format!("Could not read cloud response: {e}"))?; if bytes.len()>MAX_OUTPUT{return Err("Cloud response exceeds the 64 KiB output limit.".into());}
 let parsed:ChatResponse=serde_json::from_slice(&bytes).map_err(|e|format!("Provider response was not valid OpenAI-compatible Chat Completions JSON: {e}"))?;
 parsed.choices.into_iter().next().map(|c|c.message.content).filter(|s|!s.trim().is_empty()).ok_or_else(||"Cloud provider returned no assistant message.".into())
}
#[cfg(test)] mod tests{use super::*;#[test]fn requires_consent(){assert!(validate("https://api.example.com/v1/chat/completions","m","k","hi",false).is_err());}
#[test]fn endpoint_requires_safe_https(){assert!(validate_endpoint("http://api.example.com").is_err());assert!(validate_endpoint("https://localhost/a").is_err());assert!(validate_endpoint("https://127.0.0.1/a").is_err());assert!(validate_endpoint("https://user:pass@example.com/a").is_err());assert!(validate_endpoint("https://api.example.com/v1/chat/completions").is_ok());}
#[test]fn input_limits_are_enforced(){assert!(validate("https://api.example.com/a"," ","k","hi",true).is_err());assert!(validate("https://api.example.com/a","m","","hi",true).is_err());assert!(validate("https://api.example.com/a","m","k",&"x".repeat(MAX_PROMPT+1),true).is_err());}}
