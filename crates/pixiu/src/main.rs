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
    let web_dir = pixiu::web::locate(config.paths.web_dir.as_deref());

    let services = pixiu::Services::new(db, &config, secrets).await?;
    services.start();
    let app = pixiu::app(&services, &config, &web_dir);
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
    let stopping = services.clone();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        stopping.shut_down();
    })
    .await?;
    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM (what `docker stop` sends).
async fn shutdown_signal() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}

fn init_tracing(default_filter: &str) {
    let spec = std::env::var("RUST_LOG")
        .ok()
        .filter(|spec| !spec.trim().is_empty())
        .unwrap_or_else(|| default_filter.to_owned());
    let mut filter = EnvFilter::try_new(&spec).unwrap_or_else(|_| EnvFilter::new("info"));
    // chromiumoxide warns about every browser message newer than its
    // protocol definitions: noise, unless asked for by name.
    if !spec.contains("chromiumoxide") {
        filter = filter.add_directive("chromiumoxide=error".parse().expect("a valid directive"));
    }
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

    if answers_like_subsonic(&String::from_utf8_lossy(&response)) {
        Ok(())
    } else {
        bail!("unexpected response from {addr}");
    }
}

/// Whether an HTTP response is the Subsonic API answering. Any answer will
/// do: `ping` wants credentials, which the healthcheck does not have.
fn answers_like_subsonic(response: &str) -> bool {
    response.starts_with("HTTP/1.1 200") && response.contains("<subsonic-response ")
}

#[cfg(test)]
mod tests {
    use super::answers_like_subsonic;

    #[test]
    fn any_api_answer_is_healthy() {
        let answer =
            |body: &str| format!("HTTP/1.1 200 OK\r\ncontent-type: text/xml\r\n\r\n{body}");
        assert!(answers_like_subsonic(&answer(
            r#"<subsonic-response xmlns="http://subsonic.org/restapi" status="failed"><error code="10"/></subsonic-response>"#
        )));
        assert!(answers_like_subsonic(&answer(
            r#"<subsonic-response xmlns="http://subsonic.org/restapi" status="ok"/>"#
        )));
        assert!(!answers_like_subsonic(&answer(
            "<html>502 Bad Gateway</html>"
        )));
        assert!(!answers_like_subsonic(
            "HTTP/1.1 503 Service Unavailable\r\n\r\n"
        ));
    }
}
