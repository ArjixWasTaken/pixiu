use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    process::ExitCode,
    time::Duration,
};

use anyhow::{Context, bail};
use pixiu_core::{Config, SecretBox};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use topcoat::asset::AssetBundle;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        None | Some("serve") => serve().await,
        Some("healthcheck") => healthcheck().await,
        Some(other) => Err(anyhow::anyhow!(
            "unknown command `{other}` (expected `serve` or `healthcheck`)"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn serve() -> anyhow::Result<()> {
    let config = Config::load()?;
    init_tracing(&config.log.filter);

    for dir in [&config.paths.data_dir, &config.paths.treasure_dir] {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }
    let db = pixiu_db::open(&config.paths.database_file()).await?;
    let secrets = SecretBox::load_or_create(&config.paths.secret_key_file())?;
    let assets = AssetBundle::load().context(
        "the WebUI asset bundle is missing; build it with `topcoat asset bundle -p pixiu`",
    )?;

    let app = pixiu::app(db, &config, secrets, assets);
    let listener = TcpListener::bind((config.server.host, config.server.port))
        .await
        .with_context(|| {
            format!(
                "failed to bind {}:{}",
                config.server.host, config.server.port
            )
        })?;
    tracing::info!(
        "píxiū {} is listening on http://{}",
        pixiu_core::VERSION,
        listener.local_addr()?
    );
    topcoat::serve(listener, app).await?;
    Ok(())
}

fn init_tracing(default_filter: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Exits successfully when the local server answers `/rest/ping`. Used by
/// the container healthcheck, since the image ships no HTTP client.
async fn healthcheck() -> anyhow::Result<()> {
    let config = Config::load()?;
    let host = match config.server.host {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    let addr = SocketAddr::new(host, config.server.port);

    let check = async {
        let mut stream = TcpStream::connect(addr).await?;
        stream
            .write_all(b"GET /rest/ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        anyhow::Ok(response)
    };
    let response = tokio::time::timeout(Duration::from_secs(5), check)
        .await
        .context("timed out")??;

    let head = String::from_utf8_lossy(&response);
    if head.starts_with("HTTP/1.1 200") && head.contains(r#"status="ok""#) {
        Ok(())
    } else {
        bail!("unexpected response from {addr}");
    }
}
