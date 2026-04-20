use reqwest::Client;
use serde::Serialize;
use std::time::{Duration, Instant};

pub fn client() -> Client {
    // 10s is roomy enough for slow VPN hops without making the user feel
    // the app is stuck. Per-call retry happens at the env_checker layer.
    Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("ClaudeTool/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client should build")
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
