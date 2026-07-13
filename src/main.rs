mod api;
mod config;
mod output;
mod player;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use std::io::{self, Write};

use crate::{
    api::{ApiClient, SongQuery, validate_limit},
    config::{Config, config_path, remove_config},
};

#[derive(Debug, Parser)]
#[command(name = "blackcandy")]
#[command(version)]
#[command(about = "A command-line client for Black Candy music servers.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Log in to a Black Candy server and store an API token.
    Login(LoginArgs),
    /// Log out by removing the stored credentials.
    Logout,
    /// Show the configured server and current config path.
    Config,
    /// Show Black Candy server version information.
    System {
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Search songs, albums, artists, and playlists.
    Search {
        query: String,
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Browse or inspect songs.
    Songs(SongsArgs),
    /// Play a song by ID.
    Play(PlayArgs),
    /// Manage the current playback queue.
    Queue(QueueArgs),
    /// Manage favorite songs.
    Favorite(FavoriteArgs),
    /// Manage playlists.
    Playlist(PlaylistArgs),
}

#[derive(Debug, Args)]
struct LoginArgs {
    /// Black Candy server URL, for example http://localhost:3000. If omitted, you will be prompted.
    server: Option<String>,
    /// Account email. If omitted, you will be prompted.
    #[arg(short, long, env = "BLACKCANDY_EMAIL")]
    email: Option<String>,
    /// Account password. Prefer the prompt or BLACKCANDY_PASSWORD over shell history.
    #[arg(short, long, env = "BLACKCANDY_PASSWORD", hide_env_values = true)]
    password: Option<String>,
}

#[derive(Debug, Args)]
struct SongsArgs {
    #[command(subcommand)]
    command: Option<SongsCommand>,
}

#[derive(Debug, Subcommand)]
enum SongsCommand {
    /// List songs.
    List(SongListArgs),
    /// Show one song.
    Show {
        id: u64,
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
struct SongListArgs {
    #[arg(long)]
    album_year: Option<String>,
    #[arg(long)]
    album_genre: Option<String>,
    #[arg(long)]
    sort: Option<String>,
    #[arg(long)]
    sort_direction: Option<String>,
    #[arg(long, default_value_t = 1)]
    page: u32,
    #[arg(long, default_value_t = 30)]
    limit: u32,
    /// Print the raw JSON response.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PlayArgs {
    song_id: u64,
    /// Print the authenticated stream URL instead of playing it.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct QueueArgs {
    #[command(subcommand)]
    command: QueueCommand,
}

#[derive(Debug, Subcommand)]
enum QueueCommand {
    /// List queued songs.
    List {
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Add a song to the queue.
    Add {
        song_id: u64,
        /// Append to the end instead of inserting at the start.
        #[arg(long)]
        last: bool,
    },
    /// Clear the queue.
    Clear,
}

#[derive(Debug, Args)]
struct FavoriteArgs {
    #[command(subcommand)]
    command: FavoriteCommand,
}

#[derive(Debug, Subcommand)]
enum FavoriteCommand {
    /// List favorite songs.
    List {
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Add a song to favorites.
    Add { song_id: u64 },
    /// Remove a song from favorites.
    Remove { song_id: u64 },
}

#[derive(Debug, Args)]
struct PlaylistArgs {
    #[command(subcommand)]
    command: PlaylistCommand,
}

#[derive(Debug, Subcommand)]
enum PlaylistCommand {
    /// List playlists.
    List {
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Create a new playlist.
    Create { name: String },
    /// List songs in a playlist.
    Songs {
        playlist_id: u64,
        /// Print the raw JSON response.
        #[arg(long)]
        json: bool,
    },
    /// Add a song to a playlist.
    AddSong { playlist_id: u64, song_id: u64 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Login(args) => login(args).await,
        Command::Logout => logout().await,
        Command::Config => show_config(),
        command => {
            let config = Config::load()?;
            let client = configured_client(&config)?;
            run_authenticated(command, client).await
        }
    }
}

async fn login(args: LoginArgs) -> Result<()> {
    let server = match args.server {
        Some(server) => server,
        None => prompt_line("Server: ")?,
    };
    if server.is_empty() {
        bail!("server address is required");
    }
    let email = match args.email {
        Some(email) => email,
        None => prompt_line("Email: ")?,
    };
    let password = match args.password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    let client = ApiClient::new(&server, None)?;
    let response = client.login(&email, &password).await?;

    let config = Config {
        server: Some(client.server().as_str().to_owned()),
        email: Some(response.user.email.clone()),
        api_token: Some(response.user.api_token),
    };
    config.save()?;

    println!(
        "Logged in as {}{}.",
        response.user.email,
        if response.user.is_admin {
            " (admin)"
        } else {
            ""
        }
    );
    println!("Config saved to {}.", config_path()?.display());
    Ok(())
}

async fn logout() -> Result<()> {
    let config = Config::load()?;
    if config.api_token.is_some() {
        let client = configured_client(&config)?;
        client.logout().await?;
    }

    if remove_config()? {
        println!(
            "Logged out. Removed config at {}.",
            config_path()?.display()
        );
    } else {
        println!("Not logged in; nothing to remove.");
    }
    Ok(())
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

fn show_config() -> Result<()> {
    let config = Config::load()?;
    println!("Config: {}", config_path()?.display());
    println!(
        "Server: {}",
        config.server.as_deref().unwrap_or("<not configured>")
    );
    println!("Email: {}", config.email.as_deref().unwrap_or("<unknown>"));
    println!(
        "Token: {}",
        if config.api_token.is_some() {
            "<stored>"
        } else {
            "<missing>"
        }
    );
    Ok(())
}

async fn run_authenticated(command: Command, client: ApiClient) -> Result<()> {
    match command {
        Command::System { json } => {
            let system = client.system().await?;
            if json {
                print_json(&system)?;
            } else {
                println!("Server: {}", system.version.display());
                println!("Minimum app version: {}", system.min_app_version.display());
            }
        }
        Command::Search { query, json } => {
            let response = client.search(&query).await?;
            if json {
                print_json(&response)?;
            } else {
                output::search(&response);
            }
        }
        Command::Songs(args) => run_songs(args, &client).await?,
        Command::Play(args) => {
            let song = client.song(args.song_id).await?;
            println!("Playing: {} - {}", song.artist_name, song.name);
            player::play(&client, &song, args.dry_run).await?;
        }
        Command::Queue(args) => run_queue(args, &client).await?,
        Command::Favorite(args) => run_favorite(args, &client).await?,
        Command::Playlist(args) => run_playlist(args, &client).await?,
        Command::Login(_) | Command::Logout | Command::Config => {
            unreachable!("handled before authentication setup")
        }
    }

    Ok(())
}

async fn run_songs(args: SongsArgs, client: &ApiClient) -> Result<()> {
    match args.command.unwrap_or(SongsCommand::List(SongListArgs {
        album_year: None,
        album_genre: None,
        sort: None,
        sort_direction: None,
        page: 1,
        limit: 30,
        json: false,
    })) {
        SongsCommand::List(args) => {
            let query = SongQuery {
                album_year: args.album_year,
                album_genre: args.album_genre,
                sort: args.sort,
                sort_direction: args.sort_direction,
                page: args.page,
                limit: validate_limit(args.limit)?,
            };
            let songs = client.songs(&query).await?;
            if args.json {
                print_json(&songs)?;
            } else {
                output::songs(&songs);
            }
        }
        SongsCommand::Show { id, json } => {
            let song = client.song(id).await?;
            if json {
                print_json(&song)?;
            } else {
                output::songs(&[song]);
            }
        }
    }
    Ok(())
}

async fn run_queue(args: QueueArgs, client: &ApiClient) -> Result<()> {
    match args.command {
        QueueCommand::List { json } => {
            let songs = client.queue().await?;
            if json {
                print_json(&songs)?;
            } else {
                output::songs(&songs);
            }
        }
        QueueCommand::Add { song_id, last } => {
            let song = client.add_queue_song(song_id, last).await?;
            println!("Added to queue: {} - {}", song.artist_name, song.name);
        }
        QueueCommand::Clear => {
            client.clear_queue().await?;
            println!("Queue cleared.");
        }
    }
    Ok(())
}

async fn run_favorite(args: FavoriteArgs, client: &ApiClient) -> Result<()> {
    match args.command {
        FavoriteCommand::List { json } => {
            let songs = client.favorites().await?;
            if json {
                print_json(&songs)?;
            } else {
                output::songs(&songs);
            }
        }
        FavoriteCommand::Add { song_id } => {
            let song = client.add_favorite(song_id).await?;
            println!("Favorited: {} - {}", song.artist_name, song.name);
        }
        FavoriteCommand::Remove { song_id } => {
            let song = client.remove_favorite(song_id).await?;
            println!("Removed favorite: {} - {}", song.artist_name, song.name);
        }
    }
    Ok(())
}

async fn run_playlist(args: PlaylistArgs, client: &ApiClient) -> Result<()> {
    match args.command {
        PlaylistCommand::List { json } => {
            let playlists = client.playlists().await?;
            if json {
                print_json(&playlists)?;
            } else {
                output::playlists(&playlists);
            }
        }
        PlaylistCommand::Create { name } => {
            let playlist = client.create_playlist(&name).await?;
            println!("Created playlist {}: {}", playlist.id, playlist.name);
        }
        PlaylistCommand::Songs { playlist_id, json } => {
            let songs = client.playlist_songs(playlist_id).await?;
            if json {
                print_json(&songs)?;
            } else {
                output::songs(&songs);
            }
        }
        PlaylistCommand::AddSong {
            playlist_id,
            song_id,
        } => {
            let song = client.add_playlist_song(playlist_id, song_id).await?;
            println!(
                "Added to playlist {playlist_id}: {} - {}",
                song.artist_name, song.name
            );
        }
    }
    Ok(())
}

fn configured_client(config: &Config) -> Result<ApiClient> {
    let server = config.require_server()?;
    let token = config.require_token()?;
    ApiClient::new(server, Some(token))
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value).context("failed to serialize JSON output")?;
    println!("{raw}");
    Ok(())
}
