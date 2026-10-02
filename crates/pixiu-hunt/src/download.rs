//! Fetching audio: ranged downloads from stream URLs, with `yt-dlp` as the
//! fallback when the platform client fails.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::{
    StatusCode,
    header::{RANGE, USER_AGENT},
};
use tokio::io::AsyncWriteExt;

use crate::{HuntError, source::YtDlpTarget, ytmusic::AudioSource};

/// YouTube throttles large requests; many smaller ranges download at full
/// speed.
const CHUNK: u64 = 10 * 1024 * 1024;
const ATTEMPTS: u32 = 3;

/// Reports `(downloaded, total)` bytes.
pub(crate) type Progress<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

/// Downloads `source` to `dest`.
pub(crate) async fn fetch(
    client: &reqwest::Client,
    source: &AudioSource,
    dest: &Path,
    progress: Progress<'_>,
) -> Result<(), HuntError> {
    let mut file = tokio::fs::File::create(dest).await?;
    match source.size {
        Some(size) => {
            let mut offset = 0;
            while offset < size {
                let end = (offset + CHUNK).min(size) - 1;
                let bytes = fetch_range(client, source, offset, end).await?;
                file.write_all(&bytes).await?;
                offset = end + 1;
                progress(offset, Some(size));
            }
        }
        None => {
            let response = client
                .get(&source.url)
                .header(USER_AGENT, &source.user_agent)
                .send()
                .await?
                .error_for_status()?;
            let total = response.content_length();
            let mut done = 0;
            let mut body = response.bytes_stream();
            while let Some(chunk) = body.next().await {
                let chunk = chunk?;
                file.write_all(&chunk).await?;
                done += chunk.len() as u64;
                progress(done, total);
            }
        }
    }
    file.flush().await?;
    file.sync_all().await?;
    Ok(())
}

async fn fetch_range(
    client: &reqwest::Client,
    source: &AudioSource,
    start: u64,
    end: u64,
) -> Result<Bytes, HuntError> {
    let mut last_error = None;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(u64::from(attempt) * 2)).await;
        }
        let result = async {
            client
                .get(&source.url)
                .header(USER_AGENT, &source.user_agent)
                .header(RANGE, format!("bytes={start}-{end}"))
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await
        }
        .await;
        match result {
            Ok(bytes) if bytes.len() as u64 == end - start + 1 => return Ok(bytes),
            Ok(bytes) => {
                last_error = Some(HuntError::Download(format!(
                    "expected {} bytes, got {}",
                    end - start + 1,
                    bytes.len()
                )));
            }
            Err(error) => {
                // A refused stream stays refused; retry only what may pass.
                let refused = error.status().is_some_and(|status| {
                    status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS
                });
                last_error = Some(error.into());
                if refused {
                    break;
                }
            }
        }
    }
    Err(last_error.expect("at least one attempt ran"))
}

/// Downloads the best audio of `target` with `yt-dlp` into `dir` (the job's
/// own), returning the file it wrote.
pub(crate) async fn yt_dlp(
    target: &YtDlpTarget,
    dir: &Path,
    cookies: Option<&str>,
) -> Result<PathBuf, HuntError> {
    let prefix = "audio.yt-dlp";
    let mut command = tokio::process::Command::new("yt-dlp");
    command
        .args(["--no-playlist", "--no-progress", "--quiet", "--no-warnings"])
        .args(["--format", "bestaudio[acodec=opus]/bestaudio"])
        .arg("--output")
        .arg(dir.join(format!("{prefix}.%(ext)s")))
        .kill_on_drop(true);

    let cookie_file = match (cookies, target.cookie_domain) {
        (Some(header), Some(domain)) => {
            let path = dir.join("cookies.txt");
            write_private(&path, &netscape_cookies(header, domain)).await?;
            command.arg("--cookies").arg(&path);
            Some(path)
        }
        _ => None,
    };
    command.arg(&target.url);

    let output = command.output().await;
    if let Some(path) = &cookie_file {
        let _ = tokio::fs::remove_file(path).await;
    }
    let output = output.map_err(|error| HuntError::YtDlp(format!("cannot run yt-dlp: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last_line = stderr.lines().last().unwrap_or("no output");
        return Err(HuntError::YtDlp(plain_reason(last_line)));
    }

    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(prefix))
        {
            return Ok(entry.path());
        }
    }
    Err(HuntError::YtDlp("yt-dlp wrote no file".to_owned()))
}

/// yt-dlp's last word, for people: without its `ERROR: [youtube] <id>:`
/// prefix and its hints for the command line.
fn plain_reason(line: &str) -> String {
    let mut reason = line.trim().trim_start_matches("ERROR:").trim_start();
    // `[youtube] dQw4w9WgXcQ: …`
    if let Some(rest) = reason.strip_prefix('[')
        && let Some((_, rest)) = rest.split_once("] ")
    {
        reason = rest.split_once(": ").map_or(rest, |(_, rest)| rest);
    }
    let reason = reason
        .split_once(" Use --")
        .map_or(reason, |(reason, _)| reason)
        .trim_end_matches(',')
        .trim();
    if reason.is_empty() {
        line.trim().to_owned()
    } else {
        reason.to_owned()
    }
}

#[cfg(test)]
mod plain_reason_tests {
    use super::plain_reason;

    #[test]
    fn yt_dlp_s_reasons_lose_their_prefix_and_hints() {
        assert_eq!(
            plain_reason(
                "ERROR: [youtube] _QfPliSW83A: Requested format is not available. \
                 Use --list-formats for a list of available formats"
            ),
            "Requested format is not available."
        );
        assert_eq!(
            plain_reason("ERROR: unable to download video data: HTTP Error 403: Forbidden"),
            "unable to download video data: HTTP Error 403: Forbidden"
        );
        assert_eq!(plain_reason("no output"), "no output");
    }
}

/// Converts a `Cookie` header into the Netscape cookie file `yt-dlp` reads,
/// for `domain` (like `.youtube.com`).
fn netscape_cookies(header: &str, domain: &str) -> String {
    let mut file = String::from("# Netscape HTTP Cookie File\n");
    for pair in header.split(';') {
        if let Some((name, value)) = pair.trim().split_once('=') {
            file.push_str(&format!(
                "{domain}\tTRUE\t/\tTRUE\t0\t{}\t{}\n",
                name.trim(),
                value.trim()
            ));
        }
    }
    file
}

async fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).await?;
    file.write_all(contents.as_bytes()).await?;
    file.flush().await
}

#[cfg(test)]
mod tests {
    use super::netscape_cookies;

    #[test]
    fn converts_cookie_headers() {
        assert_eq!(
            netscape_cookies("SID=abc; HSID=d=e ;x", ".youtube.com"),
            "# Netscape HTTP Cookie File\n\
             .youtube.com\tTRUE\t/\tTRUE\t0\tSID\tabc\n\
             .youtube.com\tTRUE\t/\tTRUE\t0\tHSID\td=e\n"
        );
    }
}
