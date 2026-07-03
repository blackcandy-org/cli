# Black Candy CLI

A Rust command-line client for [Black Candy](https://github.com/blackcandy-org/blackcandy).

## Install

Install the latest release with Cargo from the Git tag:

```sh
cargo install --git https://github.com/blackcandy-org/cli --tag v0.1.0 blackcandy-cli
```

Or install with Homebrew:

```sh
brew tap blackcandy-org/cli https://github.com/blackcandy-org/cli
brew install blackcandy-cli
```

To install from a local checkout:

```sh
cargo install --path .
```

All install methods provide the `blackcandy` command.

Playback uses `mpv` by default, so install it if you want to play music directly
from the terminal:

```sh
brew install mpv
```

You can also pass a different player with `--player`.

## Quick Start

Run a local Black Candy server:

```sh
docker run --rm --name blackcandy-dev -p 3000:80 ghcr.io/blackcandy-org/blackcandy:edge
```

Log in with the default development account:

```sh
blackcandy login http://localhost:3000 --email admin@admin.com
```

The server address may omit the scheme; `localhost:3000` is treated as
`http://localhost:3000`.

The default password is `foobar`. Any argument you omit (including the server
address) is prompted for interactively, so `blackcandy login` on its own walks
through each field.

Check the server and browse songs:

```sh
blackcandy system
blackcandy songs list --limit 20
blackcandy search "love"
```

Play a song:

```sh
blackcandy play 1
```

To print the stream URL without launching a player:

```sh
blackcandy play 1 --dry-run
```

## Common Commands

```sh
blackcandy system
blackcandy search "love"
blackcandy songs list --limit 20
blackcandy songs show 1
blackcandy play 1
blackcandy queue list
blackcandy queue add 1 --last
blackcandy favorite list
blackcandy favorite add 1
blackcandy playlist list
blackcandy playlist create "Road Trip"
blackcandy playlist add-song 3 1
```

Most read commands support `--json` for scripting:

```sh
blackcandy songs list --limit 10 --json
blackcandy playlist list --json
```

## Configuration

Login stores the server URL, email, API token, and optional player command in:

```sh
blackcandy config
```

The config file is written with user-only permissions on Unix systems.

For isolated test runs, point `BLACKCANDY_CONFIG` at a temporary file:

```sh
BLACKCANDY_CONFIG=/tmp/blackcandy-cli.toml \
  blackcandy login http://localhost:3000 --email admin@admin.com --password foobar
```

Login can also read credentials from environment variables:

```sh
BLACKCANDY_EMAIL=admin@admin.com \
BLACKCANDY_PASSWORD=foobar \
  blackcandy login http://localhost:3000
```

## Development

```sh
cargo build
cargo run -- --help
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

For local development without installing the binary, prefix commands with
`cargo run --`:

```sh
cargo run -- songs list --limit 20
```
