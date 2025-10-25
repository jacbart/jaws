use aws_sdk_secretsmanager::Client;

pub async fn show_secret(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get_secret_value().secret_id(name).send().await?;

    println!("{}", resp.secret_string().unwrap_or("No value!"));

    Ok(())
}
