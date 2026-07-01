use anyhow::{Context, Result, anyhow};
use std::io::Cursor;
use std::process::Command;

use crate::api::{ApiClient, Song, ensure_absolute_url};

pub async fn play(
    client: &ApiClient,
    song: &Song,
    player: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let url = ensure_absolute_url(client.server(), &song.url)?;

    if dry_run {
        println!("{url}");
        return Ok(());
    }

    match player {
        // An explicit player command still shells out, keeping the previous
        // mpv-based behaviour available for anyone who wants it.
        Some(player) => play_external(client, &url, player),
        // Otherwise play in process with rodio, so no external player is needed.
        None => play_builtin(client, song).await,
    }
}

async fn play_builtin(client: &ApiClient, song: &Song) -> Result<()> {
    let bytes = client.stream_bytes(&song.url).await?;

    // rodio playback blocks until the track ends, so run it off the async
    // runtime's worker threads.
    tokio::task::spawn_blocking(move || decode_and_play(bytes))
        .await
        .context("playback task failed")?
}

fn decode_and_play(bytes: Vec<u8>) -> Result<()> {
    let handle = rodio::DeviceSinkBuilder::open_default_sink()
        .context("could not open the default audio output device")?;
    let player = rodio::Player::connect_new(handle.mixer());
    let source = rodio::Decoder::new(Cursor::new(bytes)).context("could not decode audio stream")?;

    player.append(source);
    player.sleep_until_end();
    Ok(())
}

fn play_external(client: &ApiClient, url: &str, player: &str) -> Result<()> {
    let token = client
        .token()
        .ok_or_else(|| anyhow!("not logged in; run `blackcandy login <server>` first"))?;
    let auth_header = format!("Authorization: Token token=\"{token}\"");

    let status = Command::new(player)
        .arg("--no-video")
        .arg("--force-window=no")
        .arg(format!("--http-header-fields={auth_header}"))
        .arg(url)
        .status()
        .with_context(|| format!("failed to start player `{player}`"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("player exited with {status}"))
    }
}
