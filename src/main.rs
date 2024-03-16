use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_lambda_events::event::eventbridge::EventBridgeEvent;
use aws_sdk_dynamodb::types::AttributeValue;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use serde_json::Value;
use std::{
    collections::HashMap,
    env,
    time::{Duration, Instant},
};

struct HandlerConfig {
    url: reqwest::Url,
    client: reqwest::Client,
    table_name: String,
    dynamodb_client: aws_sdk_dynamodb::Client,
}

async fn function_handler(
    config: &HandlerConfig,
    event: LambdaEvent<EventBridgeEvent<Value>>,
) -> Result<(), Error> {
    let start = Instant::now();
    let resp = config.client.get(config.url.as_str()).send().await;
    let duration = start.elapsed();

    let timestamp = event
        .payload
        .time
        .unwrap_or_else(chrono::Utc::now)
        .format("%+")
        .to_string();
    let mut item = HashMap::new();
    item.insert(
        "Id".to_string(),
        AttributeValue::S(format!("{}#{}", config.url, timestamp)),
    );
    item.insert("Timestamp".to_string(), AttributeValue::S(timestamp));

    let success = match resp {
        Ok(resp) => {
            let status = resp.status().as_u16();
            item.insert("Status".to_string(), AttributeValue::N(status.to_string()));
            item.insert(
                "Duration".to_string(),
                AttributeValue::N(duration.as_millis().to_string()),
            );
            resp.status().is_success()
        }
        Err(e) => {
            item.insert("Error".to_string(), AttributeValue::S(e.to_string()));
            false
        }
    };
    item.insert("Success".to_string(), AttributeValue::Bool(success));

    let insert_result = config
        .dynamodb_client
        .put_item()
        .table_name(config.table_name.as_str())
        .set_item(Some(item))
        .send()
        .await?;

    tracing::info!("Insert result: {:?}", insert_result);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let url = env::var("URL").expect("URL environment variable is not set");
    let url = reqwest::Url::parse(&url).expect("URL environment variable is not a valid URL");
    let timeout = env::var("TIMEOUT").unwrap_or_else(|_| "60".to_string());
    let timeout = timeout
        .parse::<u64>()
        .expect("TIMEOUT environment variable is not a valid number");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME environment variable is not set");

    let region_provider = RegionProviderChain::default_provider();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

    let config = &HandlerConfig {
        url,
        client,
        table_name,
        dynamodb_client,
    };

    run(service_fn(move |event| async move {
        function_handler(config, event).await
    }))
    .await
}
