use crate::config;
use reqwest::Client;
use serde::Serialize;
use std::time::{Duration, Instant};

pub fn client() -> Client {
    let cfg = config::load();
    // 30s roomy enough for slow VPN hops + large file downloads. Per-call
    // retry with exponential backoff happens at the env_checker / installer layer.
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("ClaudeTool/", env!("CARGO_PKG_VERSION")));

    // ADR-018: honor user-configured proxy for in-process HTTP (network probe,
    // GitHub Releases API, Git installer download).
    // Use Proxy::all() so the proxy survives 302 redirects to other subdomains
    // (e.g. github.com → objects.githubusercontent.com for release assets).
    let proxy_url = cfg
        .proxy
        .https
        .as_ref()
        .filter(|s| !s.is_empty())
        .or(cfg.proxy.http.as_ref().filter(|s| !s.is_empty()));
    if let Some(url) = proxy_url {
        if let Ok(p) = reqwest::Proxy::all(url) {
            builder = builder.proxy(p);
        }
    }

    builder.build().expect("reqwest client should build")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub status: Option<u16>,
    pub error: Option<String>,
}

pub async fn probe(url: &str) -> ProbeResult {
    let start = Instant::now();
    match client().head(url).send().await {
        Ok(resp) => ProbeResult {
            reachable: true,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            status: Some(resp.status().as_u16()),
            error: None,
        },
        Err(e) => ProbeResult {
            reachable: false,
            latency_ms: None,
            status: None,
            error: Some(e.to_string()),
        },
    }
}
