use anyhow::{Context, Result, anyhow};
use std::process::Command;

use crate::api::{ApiClient, Song, ensure_absolute_url};

pub fn play(client: &ApiClient, song: &Song, player: Option<&str>, dry_run: bool) -> Result<()> {
    let url = ensure_absolute_url(client.server(), &song.url)?;
    let token = client
        .token()
        .ok_or_else(|| anyhow!("not logged in; run `blackcandy login <server>` first"))?;
    let auth_header = format!("Authorization: Token token=\"{token}\"");

    if dry_run {
        println!("{url}");
        return Ok(());
    }

    let player = match player {
        Some(player) => player.to_owned(),
        None => which::which("mpv")
            .map(|path| path.display().to_string())
            .context("could not find mpv; install mpv or use --player")?,
    };

    let status = Command::new(&player)
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
