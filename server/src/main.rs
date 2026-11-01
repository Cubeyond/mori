//! Command-line bootstrap, contract export, and HTTP server lifecycle.

use clap::{Parser, Subcommand};
use mori::{
    app::{AppState, Config},
    features::auth,
    http,
    infra::db,
    jobs,
};

/// Mori command-line entry point for serving, bootstrap, and contract export.
#[derive(Parser)]
struct Cli {
    /// Requested command-line operation.
    #[command(subcommand)]
    command: Command,
}
/// Administrative operations available without a separate control service.
#[derive(Subcommand)]
enum Command {
    /// Serve the immutable frontend and authenticated HTTP API.
    Serve(Box<Config>),
    /// Export the deterministic generated API contract.
    Openapi {
        /// Destination path for the generated YAML contract.
        #[arg(long, default_value = "api/openapi.yml")]
        output: String,
    },
    /// Initialize or reset the single administrator password from stdin.
    SetPassword {
        #[arg(
            long,
            env = "MORI_DATABASE_URL",
            default_value = "sqlite://data/mori.db"
        )]
        /// SQLite metadata connection URL.
        database_url: String,
    },
}

/// Run administrator bootstrap, contract generation, or the bounded HTTP service.
///
/// # Errors
/// Returns invalid configuration, bootstrap I/O, contract export, database, or server errors.
#[tokio::main(worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    match Cli::parse().command {
        Command::Openapi { output } => {
            let spec = http::openapi().to_yaml()?;
            if let Some(parent) = std::path::Path::new(&output).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output, spec)?;
        }
        Command::SetPassword { database_url } => {
            use std::io::Read;
            eprintln!("Read administrator password from stdin (minimum 12 characters).");
            let mut password = String::new();
            std::io::stdin().read_to_string(&mut password)?;
            let password = password.trim_end_matches(['\r', '\n']);
            anyhow::ensure!(
                password.len() >= 12 && password.len() <= 1024,
                "password must be 12..=1024 bytes"
            );
            let hash = auth::hash_password(password)?;
            let db = db::connect(&database_url).await?;
            let mut tx = db.begin().await?;
            sqlx::query(
                r"INSERT INTO admin(id, password_hash) VALUES(1, ?) ON CONFLICT(id) DO UPDATE
SET password_hash=excluded.password_hash",
            )
            .bind(hash)
            .execute(&mut *tx)
            .await?;
            sqlx::query("DELETE FROM sessions")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            eprintln!("Administrator password updated; sessions revoked.");
        }
        Command::Serve(config) => {
            let state = AppState::new(*config).await?;
            let listener = tokio::net::TcpListener::bind(&state.config.listen).await?;
            #[cfg(unix)]
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            let worker = tokio::spawn(jobs::run(state.clone()));
            axum::serve(listener, http::router(state))
                .with_graceful_shutdown(async move {
                    #[cfg(unix)]
                    {
                        tokio::select! {_ = tokio::signal::ctrl_c()=>{},_ = terminate.recv()=>{}}
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = tokio::signal::ctrl_c().await;
                    }
                })
                .await?;
            worker.abort();
        }
    }
    Ok(())
}
