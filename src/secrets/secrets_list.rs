use aws_sdk_secretsmanager::{Client, types::Filter};
use ff::{TuiConfig, create_items_channel, run_tui_with_config};

pub async fn _list_all(
    client: &Client,
    filters: Option<Vec<Filter>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let builder = client.list_secrets().set_filters(filters).into_paginator();
    let mut stream = builder.send();

    while let Some(page) = stream.next().await {
        let list = page.unwrap().secret_list.unwrap();
        list.iter()
            .for_each(|secret| println!("{}", secret.name.as_ref().unwrap()));
    }

    Ok(())
}

pub async fn tui_selector(
    client: &Client,
    filters: Option<Vec<Filter>>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let builder = client.list_secrets().set_filters(filters).into_paginator();
    let mut stream = builder.send();

    let (tx, rx) = create_items_channel();

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(page) = stream.next().await {
            let list = match page {
                Ok(p) => p.secret_list.unwrap(),
                Err(_) => break,
            };
            for secret in list.iter() {
                let secret_name = secret.name.to_owned().unwrap_or_else(|| "".to_string());
                let _ = tx_clone.send(secret_name).await;
            }
        }
    });

    let mut config = TuiConfig::fullscreen();
    config.show_help_text = false;
    let sel = run_tui_with_config(rx, true, config).await?;
    Ok(sel)
}
