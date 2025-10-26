mod secrets;
mod secrets_list;

use std::process::Command;
use std::{env, path::PathBuf};

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
    // set editor to open
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    // let editor = String::from("vim");

    // set path where secrets will be downloaded
    let path = PathBuf::from("./.secrets");
    let mut files: Vec<String> = vec![];

    let secret_list = secrets_list::tui_selector(&client, None).await?;
    for secret in secret_list {
        // secrets::print_secret(&client, secret.as_str()).await?;
        let file_path = secrets::download_secret(&client, secret.as_str(), path.to_owned()).await?;
        println!("{secret} -> {file_path}");
        files.push(file_path);
    }

    if files.len() > 0 {
        let _ = Command::new(&editor)
            .args(&files)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}
