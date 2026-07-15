---
name: blackcandy
description: Interact with a Black Candy music server through the blackcandy CLI. Use for any request to search or browse music, inspect songs and playlists, play audio, manage the playback queue, manage favorite songs, create playlists, add songs to playlists, check server status, or configure Black Candy access.
---

# Black Candy CLI

Use the `blackcandy` command to work with the user's Black Candy music server.

## Agent rules

1. Run `blackcandy config` when setup is uncertain. If the server or token is missing, ask the user for the server and email, then run `blackcandy login`; let the password prompt securely rather than putting a password in shell history.
2. Prefer `--json` on read commands so IDs and metadata remain unambiguous. Use the normal table output only for concise user-facing display.
3. Resolve song and playlist IDs with `search`, `songs list`, or `playlist list` before running a mutation. Never guess an ID.
4. Treat `queue clear` as destructive and run it only when the user explicitly requests clearing the queue or confirms the action.
5. `play` produces audible output. Run it only when playback is requested; use `--dry-run` when the user only needs the authenticated stream URL.
6. Never read or expose the stored API token. `blackcandy config` reports whether a token exists without printing it.

## Workflow

Check connectivity or diagnose setup:

```sh
blackcandy config
blackcandy system --json
```

Find music before acting on it:

```sh
blackcandy search "query" --json
blackcandy songs list --limit 20 --json
blackcandy songs show SONG_ID --json
```

Then use the returned numeric IDs with the requested action:

```sh
blackcandy play SONG_ID
blackcandy queue add SONG_ID --last
blackcandy favorite add SONG_ID
blackcandy playlist add-song PLAYLIST_ID SONG_ID
```

After a mutation, report the affected song or playlist and the action completed. If a command fails, relay the CLI error and use `blackcandy config`, `blackcandy system`, or `blackcandy --help` to diagnose it instead of guessing.

## Command reference

### Setup and server

```sh
blackcandy login [SERVER] [--email EMAIL]
blackcandy config
blackcandy system [--json]
```

`login` prompts for any omitted value. It also accepts `BLACKCANDY_EMAIL` and `BLACKCANDY_PASSWORD`; prefer the password prompt when operating interactively.

### Search and songs

```sh
blackcandy search QUERY [--json]
blackcandy songs list [--album-year YEAR] [--album-genre GENRE] \
  [--sort FIELD] [--sort-direction DIRECTION] [--page N] [--limit N] [--json]
blackcandy songs show SONG_ID [--json]
blackcandy play SONG_ID [--dry-run]
```

Search returns songs, albums, artists, and playlists. Song list limits must be between 1 and 100.

### Queue and favorites

```sh
blackcandy queue list [--json]
blackcandy queue add SONG_ID [--last]
blackcandy queue clear
blackcandy favorite list [--json]
blackcandy favorite add SONG_ID
blackcandy favorite remove SONG_ID
```

Without `--last`, `queue add` inserts the song at the start of the queue.

### Playlists

```sh
blackcandy playlist list [--json]
blackcandy playlist create NAME
blackcandy playlist songs PLAYLIST_ID [--json]
blackcandy playlist add-song PLAYLIST_ID SONG_ID
```

Quote names and search queries containing spaces.

## Output

Read commands marked above support `--json` and print the raw server response. Lists are JSON arrays. Mutations print a short confirmation. Use the IDs from command output for follow-up operations.
