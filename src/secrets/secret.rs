use aws_sdk_secretsmanager::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// get_secret returns the secret's value as a String
pub async fn get_secret(client: &Client, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    let secret_value = resp.secret_string().expect("missing secret value");

    Ok(secret_value.to_string())
}

/// print_secret prints the secret's value to stdout
pub async fn _print_secret(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let secret_value = get_secret(client, name).await?;
    println!("{}", secret_value);

    Ok(())
}

/// download_secret gets a secret's value and saves it to a file locally, returning the file path
pub async fn download_secret(
    client: &Client,
    name: &str,
    dir: PathBuf,
) -> Result<String, Box<dyn std::error::Error>> {
    let secret_value = get_secret(client, name).await?;
    let path = dir.join(Path::new(name));
    let path_string = path.display().to_string();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path.as_path())?;

    file.write_all(secret_value.as_bytes())?;
    Ok(path_string)
}
