use std::{env, fs, sync::Arc};

use async_trait::async_trait;
use memorynexus::reference_adapter::{
    AdapterError, GatewayAcknowledgement, GatewayAcknowledgementStatus, GatewayClient,
    NormalizedGatewayRequest, Normalizer, ReferenceAdapter, SourceClient, SourcePage, SourceRecord,
    SystemClock,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureSource {
    pages: Vec<FixturePage>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixturePage {
    after_cursor: Option<String>,
    page: SourcePage,
}

#[async_trait]
impl SourceClient for FixtureSource {
    async fn acquire_page(
        &self,
        after_cursor: Option<&str>,
        _limit: usize,
    ) -> Result<SourcePage, AdapterError> {
        self.pages
            .iter()
            .find(|entry| entry.after_cursor.as_deref() == after_cursor)
            .map(|entry| entry.page.clone())
            .ok_or(AdapterError::Acquisition)
    }
}

struct PassThroughNormalizer;

#[async_trait]
impl Normalizer for PassThroughNormalizer {
    async fn normalize(
        &self,
        record: SourceRecord,
    ) -> Result<NormalizedGatewayRequest, AdapterError> {
        serde_json::from_value(record.payload).map_err(|_| AdapterError::Normalization)
    }
}

struct HttpGateway {
    client: reqwest::Client,
    url: String,
    token: String,
}

#[async_trait]
impl GatewayClient for HttpGateway {
    async fn deliver(
        &self,
        payload: &NormalizedGatewayRequest,
    ) -> Result<GatewayAcknowledgement, AdapterError> {
        let response = self
            .client
            .post(&self.url)
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
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| AdapterError::InvalidData("missing Gateway disposition".to_string()))?;
        if !matches!(
            status,
            "learning_attempt_accepted"
                | "learning_attempt_superseded"
                | "learning_session_accepted"
                | "learning_session_superseded"
                | "learner_journey_summary_accepted"
                | "learner_journey_summary_superseded"
                | "source_tombstone_withdrawn"
        ) {
            return Err(AdapterError::InvalidData(
                "Gateway disposition is not terminal for source delivery".to_string(),
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
            // Full Source Evidence replay returns the original acknowledgement. Both
            // accepted and replayed acknowledgements are therefore terminal here.
            status: GatewayAcknowledgementStatus::Accepted,
            source_identity,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let ledger_url = args.next().ok_or(
        "usage: memorynexus-reference-adapter <sqlite-url> <source-pages.json> <gateway-url> [max-pages]",
    )?;
    let source_path = args.next().ok_or("missing source-pages.json")?;
    let gateway_url = args.next().ok_or("missing gateway-url")?;
    let max_pages = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    if args.next().is_some() || max_pages == 0 || max_pages > 100 {
        return Err("invalid bounded page count".into());
    }
    let token =
        env::var("MEMORYNEXUS_SOURCE_TOKEN").map_err(|_| "MEMORYNEXUS_SOURCE_TOKEN is required")?;
    let source: FixtureSource = serde_json::from_slice(&fs::read(source_path)?)?;
    let adapter = ReferenceAdapter::open(
        &ledger_url,
        Arc::new(source),
        Arc::new(PassThroughNormalizer),
        Arc::new(HttpGateway {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
            url: gateway_url,
            token,
        }),
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
