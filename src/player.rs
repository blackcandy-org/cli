use anyhow::{Context, Result};
use std::io::Cursor;

use crate::api::{ApiClient, Song, ensure_absolute_url};

pub async fn play(client: &ApiClient, song: &Song, dry_run: bool) -> Result<()> {
    if dry_run {
        let url = ensure_absolute_url(client.server(), &song.url)?;
        println!("{url}");
        return Ok(());
    }

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
    let source =
        rodio::Decoder::new(Cursor::new(bytes)).context("could not decode audio stream")?;

    player.append(source);
    player.sleep_until_end();
    Ok(())
}
