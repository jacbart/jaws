//! jaws CLI - A tool for managing secrets from multiple providers.

use std::fs;

use clap::Parser;

use jaws::DbProvider;
use jaws::cli::{Cli, Commands, ConfigCommands};
use jaws::commands::{
    handle_clean, handle_clear_cache, handle_create, handle_default_command, handle_delete,
    handle_export, handle_history, handle_import, handle_interactive_generate, handle_list,
    handle_log, handle_pull, handle_pull_inject, handle_push, handle_rollback, handle_sync,
};
use jaws::config::Config;
use jaws::db::{SecretRepository, init_db};
use jaws::secrets::detect_providers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize default crypto provider for rustls (required by bitwarden sdk)
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();

    // Load config from file (use CLI-specified path if provided)
    let config = Config::load_from(cli.config.config_path.as_deref())?;

    // Handle Version command separately as it doesn't require providers or database
    if let Some(Commands::Version) = &cli.command {
        println!("v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Handle Config command separately as it doesn't require providers or database
    if let Some(Commands::Config { command }) = &cli.command {
        match command {
            // No subcommand: show current configuration
            None => {
                // Show config file path
                if let Some(path) = Config::find_existing_config() {
                    println!("Config file: {}", path.display());
                } else {
                    println!("Config file: (using defaults, no config file found)");
                }
                println!();
                println!("Settings:");
                println!("  editor: {}", config.editor());
                println!("  secrets_path: {}", config.secrets_path().display());
                println!("  cache_ttl: {}s", config.cache_ttl());
                println!("  keychain_cache: {}", config.keychain_cache());
                if let Some(max_v) = config.max_versions() {
                    println!("  max_versions: {}", max_v);
                }
                if let Some(default_provider) = config.default_provider() {
                    println!("  default_provider: {}", default_provider);
                }
                println!();
                if config.providers.is_empty() {
                    println!("Providers: (none configured)");
                    println!();
                    println!("Run 'jaws config init' to set up providers.");
                } else {
                    println!("Providers:");
                    for p in &config.providers {
                        println!("  {} ({})", p.id, p.kind);
                        if let Some(profile) = &p.profile {
                            println!("    profile: {}", profile);
                        }
                        if let Some(region) = &p.region {
                            println!("    region: {}", region);
                        }
                        if let Some(vault) = &p.vault {
                            println!("    vault: {}", vault);
                        }
                        if let Some(token_env) = &p.token_env {
                            println!("    token_env: {}", token_env);
                        }
                    }
                }
                return Ok(());
            }
            Some(ConfigCommands::Init {
                path,
                overwrite,
                minimal,
            }) => {
                if *minimal {
                    // Non-interactive: generate minimal config
                    let config_path = Config::generate_config_file(path.clone(), *overwrite)?;
                    println!("Config file generated at: {}", config_path.display());
                } else {
                    // Interactive (default)
                    return handle_interactive_generate(path.clone(), *overwrite).await;
                }
                return Ok(());
            }
            Some(ConfigCommands::Get { key }) => {
                match config.get_default(key) {
                    Ok(value) => println!("{}", value),
                    Err(e) => eprintln!("{}", e),
                }
                return Ok(());
            }
            Some(ConfigCommands::Set { key, value }) => {
                let config_path = match Config::find_existing_config() {
                    Some(path) => path,
                    None => {
                        eprintln!("Config file not found. Run 'jaws config init' first.");
                        return Ok(());
                    }
                };
                let mut config = config;
                match config.set_default(key, value) {
                    Ok(()) => {
                        config.save(&config_path)?;
                        println!("Updated {} = {} in {}", key, value, config_path.display());
                    }
                    Err(e) => eprintln!("{}", e),
                }
                return Ok(());
            }
            Some(ConfigCommands::ClearCache) => {
                return handle_clear_cache(&config);
            }
        }
    }

    // Handle Export command separately (doesn't require providers)
    if let Some(Commands::Export {
        ssh_key,
        output,
        delete,
    }) = &cli.command
    {
        return handle_export(&config, ssh_key.clone(), output.clone(), *delete).await;
    }

    // Handle Import command separately (doesn't require providers)
    if let Some(Commands::Import {
        archive,
        ssh_key,
        delete,
    }) = &cli.command
    {
        return handle_import(&config, archive, ssh_key.clone(), *delete).await;
    }

    // Ensure secrets directory exists
    fs::create_dir_all(config.secrets_path())?;

    // Initialize database
    let conn = init_db(&config.db_path())?;
    let repo = SecretRepository::new(conn);

    // Handle no command - default behavior (edit downloaded secrets)
    if cli.command.is_none() {
        return handle_default_command(&config, &repo).await;
    }

    // Detect and initialize all available providers (jaws is always available)
    // Pass the repository so stored credentials can be used as fallback
    let providers = detect_providers(&config, Some(&repo)).await?;

    // Ensure providers are registered in the database
    for provider in &providers {
        repo.upsert_provider(&DbProvider {
            id: provider.id().to_string(),
            kind: provider.kind().to_string(),
            last_sync_at: None,
            config_json: None,
        })?;
    }

    match cli.command.unwrap() {
        Commands::Config { .. } => unreachable!(),

        Commands::Pull {
            secret_name,
            edit,
            print,
            inject,
            output,
        } => {
            // Validate mutually exclusive options
            if inject.is_some() && print {
                return Err("--inject and --print are mutually exclusive".into());
            }
            if inject.is_some() && edit {
                return Err("--inject and --edit are mutually exclusive".into());
            }
            if output.is_some() && inject.is_none() {
                return Err("--output can only be used with --inject".into());
            }

            // Handle inject mode
            if let Some(template_path) = inject {
                handle_pull_inject(
                    &config,
                    &repo,
                    &providers,
                    &template_path,
                    output.as_deref(),
                )
                .await?;
            } else {
                handle_pull(&config, &repo, &providers, secret_name, edit, print).await?;
            }
        }

        Commands::List { provider, local } => {
            handle_list(&config, &repo, provider, local)?;
        }

        Commands::Push { secret_name, edit } => {
            handle_push(&config, &repo, &providers, secret_name, edit).await?;
        }

        Commands::Delete {
            secret_name,
            scope,
            force,
        } => {
            handle_delete(&config, &repo, &providers, secret_name, scope, force).await?;
        }

        Commands::Sync => {
            handle_sync(&config, &repo, &providers).await?;
        }

        Commands::History {
            secret_name,
            verbose,
            limit,
            remote,
        } => {
            handle_history(
                &config,
                &repo,
                &providers,
                secret_name,
                verbose,
                limit,
                remote,
            )
            .await?;
        }

        Commands::Rollback {
            secret_name,
            version,
            edit,
            remote,
            version_id,
        } => {
            handle_rollback(
                &config,
                &repo,
                &providers,
                secret_name,
                version,
                edit,
                remote,
                version_id,
            )
            .await?;
        }

        Commands::Export { .. } => unreachable!(),
        Commands::Import { .. } => unreachable!(),
        Commands::Version => unreachable!(),

        Commands::Create {
            name,
            description,
            file,
        } => {
            handle_create(&config, &providers, name, description, file).await?;
        }

        Commands::Log { limit, provider } => {
            handle_log(&config, limit, provider).await?;
        }

        Commands::Clean {
            force,
            dry_run,
            keep_local,
        } => {
            handle_clean(&config, &repo, force, dry_run, keep_local)?;
        }
    }

    Ok(())
}
