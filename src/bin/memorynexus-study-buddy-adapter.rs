use std::{collections::HashSet, env, fs, sync::Arc};

use async_trait::async_trait;
use memorynexus::{
    reference_adapter::{
        AdapterError, GatewayAcknowledgement, GatewayAcknowledgementStatus, GatewayClient,
        NormalizedGatewayRequest, ReferenceAdapter, SystemClock,
    },
    study_buddy_adapter::loopback_http_client,
    study_buddy_adapter::{StudyBuddyAdapterConfig, StudyBuddyNormalizer, StudyBuddySourceClient},
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterConfigFile {
    actor_id: Uuid,
    space_id: Uuid,
    allowed_subject_refs: HashSet<String>,
    adapter_version: String,
}

struct HttpSurfaceGateway {
    client: reqwest::Client,
    url: reqwest::Url,
    token: String,
}

impl HttpSurfaceGateway {
    fn new(url: &str, token: String) -> Result<Self, AdapterError> {
        if token.trim().is_empty() {
            return Err(AdapterError::InvalidData(
                "Surface Gateway must be authenticated loopback HTTP".to_string(),
            ));
        }
        let (client, url) = loopback_http_client(url, "/api/v1/surfaces")?;
        Ok(Self { client, url, token })
    }
}

#[async_trait]
impl GatewayClient for HttpSurfaceGateway {
    async fn deliver(
        &self,
        payload: &NormalizedGatewayRequest,
    ) -> Result<GatewayAcknowledgement, AdapterError> {
        let response = self
            .client
            .post(self.url.clone())
            .bearer_auth(&self.token)
            .json(payload)
            .send()
            .await
            .map_err(|_| AdapterError::Delivery)?;
        if !response.status().is_success() {
            return Err(AdapterError::Delivery);
        }
        let body: Value = response.json().await.map_err(|_| AdapterError::Delivery)?;
        let result = body
            .pointer("/data/result")
            .ok_or_else(|| AdapterError::InvalidData("missing Gateway result".to_string()))?;
        let disposition = result
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| AdapterError::InvalidData("missing Gateway disposition".to_string()))?;
        if !matches!(
            disposition,
            "learning_attempt_accepted"
                | "learning_attempt_superseded"
                | "learning_session_accepted"
                | "learning_session_superseded"
                | "source_tombstone_withdrawn"
        ) {
            return Err(AdapterError::InvalidData(
                "Gateway disposition is not terminal for Study Buddy delivery".to_string(),
            ));
        }
        let source_identity =
            serde_json::from_value(result.get("source_identity").cloned().ok_or_else(|| {
                AdapterError::InvalidData("missing Gateway Source Identity".to_string())
            })?)
            .map_err(|_| {
                AdapterError::InvalidData("invalid Gateway Source Identity".to_string())
            })?;
        Ok(GatewayAcknowledgement {
            status: GatewayAcknowledgementStatus::Accepted,
            source_identity,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let ledger_url = args.next().ok_or(
        "usage: memorynexus-study-buddy-adapter <sqlite-url> <config.json> <source-feed-url> <gateway-url> [max-pages]",
    )?;
    let config_path = args.next().ok_or("missing config.json")?;
    let source_feed_url = args.next().ok_or("missing source-feed-url")?;
    let gateway_url = args.next().ok_or("missing gateway-url")?;
    let max_pages = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    if args.next().is_some() || max_pages == 0 || max_pages > 100 {
        return Err("invalid bounded page count".into());
    }
    let source_token =
        env::var("STUDY_BUDDY_SOURCE_TOKEN").map_err(|_| "STUDY_BUDDY_SOURCE_TOKEN is required")?;
    let gateway_token =
        env::var("MEMORYNEXUS_SOURCE_TOKEN").map_err(|_| "MEMORYNEXUS_SOURCE_TOKEN is required")?;
    let config_file: AdapterConfigFile = serde_json::from_slice(&fs::read(config_path)?)?;
    let config = StudyBuddyAdapterConfig::new(
        config_file.actor_id,
        config_file.space_id,
        config_file.allowed_subject_refs,
        config_file.adapter_version,
    )?;
    let source = StudyBuddySourceClient::new(
        &source_feed_url,
        source_token,
        config.subject_ref().to_string(),
    )?;
    let gateway = HttpSurfaceGateway::new(&gateway_url, gateway_token)?;
    let adapter = ReferenceAdapter::open(
        &ledger_url,
        Arc::new(source),
        Arc::new(StudyBuddyNormalizer::new(config)),
        Arc::new(gateway),
        Arc::new(SystemClock),
        100,
    )
    .await?;
    let summary = adapter.run_bounded(max_pages).await?;
    println!(
        "acquired={} acknowledged={} cursor_advanced={} has_more={}",
        summary.acquired,
        summary.acknowledged,
        summary.cursor.is_some(),
        summary.has_more
    );
    Ok(())
}
