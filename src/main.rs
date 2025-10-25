mod secrets;
mod secrets_list;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::{Client, config::Region};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let region: Option<String> = None;
    let region_provider = RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-west-2"));
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&shared_config);
    // secrets_list::list_all(&client, None).await
    let secret_list = secrets_list::tui_selector(&client, None).await?;
    for secret in secret_list {
        secrets::show_secret(&client, secret.as_str()).await?;
    }
    Ok(())
}
