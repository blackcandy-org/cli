use anyhow::{Context, Result, anyhow, bail};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use url::Url;

#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: Url,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(server: &str, token: Option<String>) -> Result<Self> {
        let mut base_url = parse_server_url(server)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }

        Ok(Self {
            http: reqwest::Client::new(),
            base_url,
            token,
        })
    }

    pub fn server(&self) -> &Url {
        &self.base_url
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse> {
        self.post_without_auth(
            "sessions",
            &json!({
                "session": {
                    "email": email,
                    "password": password
                }
            }),
        )
        .await
    }

    pub async fn system(&self) -> Result<SystemInfo> {
        self.get_without_auth("system", &[]).await
    }

    pub async fn songs(&self, query: &SongQuery) -> Result<Vec<Song>> {
        let mut params = vec![];
        if let Some(year) = &query.album_year {
            params.push(("filter[album_year]", year.as_str()));
        }
        if let Some(genre) = &query.album_genre {
            params.push(("filter[album_genre]", genre.as_str()));
        }
        if let Some(sort) = &query.sort {
            params.push(("sort", sort.as_str()));
        }
        if let Some(sort_direction) = &query.sort_direction {
            params.push(("sort_direction", sort_direction.as_str()));
        }
        let page = query.page.to_string();
        let limit = query.limit.to_string();
        params.push(("page", page.as_str()));
        params.push(("limit", limit.as_str()));

        self.get("songs", &params).await
    }

    pub async fn song(&self, id: u64) -> Result<Song> {
        self.get(&format!("songs/{id}"), &[]).await
    }

    /// Download the raw bytes of an audio stream, authenticated with the stored
    /// token. `url` may be absolute or relative to the server.
    pub async fn stream_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let absolute = ensure_absolute_url(&self.base_url, url)?;
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| anyhow!("not logged in; run `blackcandy login <server>` first"))?;

        let response = self
            .http
            .get(absolute)
            .header(AUTHORIZATION, format!("Token token=\"{token}\""))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(read_error(response).await);
        }

        let bytes = response
            .bytes()
            .await
            .context("failed to download audio stream")?;
        Ok(bytes.to_vec())
    }

    pub async fn search(&self, query: &str) -> Result<SearchResponse> {
        self.get("search", &[("query", query)]).await
    }

    pub async fn playlists(&self) -> Result<Vec<Playlist>> {
        self.get("playlists", &[]).await
    }

    pub async fn create_playlist(&self, name: &str) -> Result<Playlist> {
        self.post("playlists", &json!({ "playlist": { "name": name } }))
            .await
    }

    pub async fn playlist_songs(&self, playlist_id: u64) -> Result<Vec<Song>> {
        self.get(&format!("playlists/{playlist_id}/songs"), &[])
            .await
    }

    pub async fn add_playlist_song(&self, playlist_id: u64, song_id: u64) -> Result<Song> {
        self.post(
            &format!("playlists/{playlist_id}/songs"),
            &json!({ "song_id": song_id }),
        )
        .await
    }

    pub async fn queue(&self) -> Result<Vec<Song>> {
        self.get("current_playlist/songs", &[]).await
    }

    pub async fn add_queue_song(&self, song_id: u64, last: bool) -> Result<Song> {
        let mut body = json!({ "song_id": song_id });
        if last {
            body["location"] = json!("last");
        }

        self.post("current_playlist/songs", &body).await
    }

    pub async fn clear_queue(&self) -> Result<()> {
        self.delete_empty("current_playlist/songs").await
    }

    pub async fn favorites(&self) -> Result<Vec<Song>> {
        self.get("favorite_playlist/songs", &[]).await
    }

    pub async fn add_favorite(&self, song_id: u64) -> Result<Song> {
        self.post("favorite_playlist/songs", &json!({ "song_id": song_id }))
            .await
    }

    pub async fn remove_favorite(&self, song_id: u64) -> Result<Song> {
        self.delete(&format!("favorite_playlist/songs/{song_id}"))
            .await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, params: &[(&str, &str)]) -> Result<T> {
        self.request(self.http.get(self.url(path)?).query(params), true)
            .await
    }

    async fn get_without_auth<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        self.request(self.http.get(self.url(path)?).query(params), false)
            .await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(self.http.post(self.url(path)?).json(body), true)
            .await
    }

    async fn post_without_auth<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(self.http.post(self.url(path)?).json(body), false)
            .await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(self.http.delete(self.url(path)?), true).await
    }

    async fn delete_empty(&self, path: &str) -> Result<()> {
        let request = self
            .with_headers(self.http.delete(self.url(path)?), true)?
            .build()?;
        let response = self.http.execute(request).await?;
        if response.status().is_success() {
            return Ok(());
        }

        Err(read_error(response).await)
    }

    async fn request<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
        with_auth: bool,
    ) -> Result<T> {
        let request = self.with_headers(builder, with_auth)?.build()?;
        let response = self.http.execute(request).await?;

        if !response.status().is_success() {
            return Err(read_error(response).await);
        }

        response
            .json::<T>()
            .await
            .context("failed to decode Black Candy response")
    }

    fn with_headers(
        &self,
        builder: reqwest::RequestBuilder,
        with_auth: bool,
    ) -> Result<reqwest::RequestBuilder> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if with_auth {
            let token = self
                .token
                .as_ref()
                .ok_or_else(|| anyhow!("not logged in; run `blackcandy login <server>` first"))?;
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Token token=\"{token}\""))
                    .context("stored API token is invalid")?,
            );
        }

        Ok(builder.headers(headers))
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .with_context(|| format!("failed to build URL for {path}"))
    }
}

async fn read_error(response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    if text.is_empty() {
        return anyhow!("Black Candy returned {status}");
    }

    if let Ok(api_error) = serde_json::from_str::<ApiError>(&text) {
        return anyhow!("Black Candy returned {status}: {}", api_error.message);
    }

    anyhow!("Black Candy returned {status}: {text}")
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub user: UserWithToken,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserWithToken {
    pub id: u64,
    pub email: String,
    pub is_admin: bool,
    pub api_token: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemInfo {
    pub version: VersionInfo,
    pub min_app_version: VersionInfo,
    #[serde(default)]
    pub min_cli_version: Option<VersionInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VersionInfo {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    #[serde(default)]
    pub pre: Option<String>,
}

impl VersionInfo {
    pub fn display(&self) -> String {
        match &self.pre {
            Some(pre) if !pre.is_empty() => {
                format!("{}.{}.{}-{pre}", self.major, self.minor, self.patch)
            }
            _ => format!("{}.{}.{}", self.major, self.minor, self.patch),
        }
    }

    pub fn semver(&self) -> Result<semver::Version> {
        semver::Version::parse(&self.display())
            .with_context(|| format!("invalid version returned by server: {}", self.display()))
    }
}

#[derive(Debug, Default)]
pub struct SongQuery {
    pub album_year: Option<String>,
    pub album_genre: Option<String>,
    pub sort: Option<String>,
    pub sort_direction: Option<String>,
    pub page: u32,
    pub limit: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Song {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub duration: Option<f64>,
    pub album_id: u64,
    pub artist_id: u64,
    pub url: String,
    pub album_name: String,
    pub artist_name: String,
    #[serde(default)]
    pub is_favorited: bool,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub album_image_urls: ImageUrls,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ImageUrls {
    #[serde(default)]
    pub small: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub large: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Album {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub year: Option<u64>,
    #[serde(default)]
    pub genre: Option<String>,
    pub artist_id: u64,
    pub artist_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artist {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub is_various: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub user_id: u64,
    #[serde(default)]
    pub is_favorite: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub playlists: Vec<Playlist>,
    #[serde(default)]
    pub songs: Vec<Song>,
}

/// Parse a user-supplied server address, defaulting to the `http` scheme when
/// none is given so both `localhost:3000` and `http://localhost:3000` work.
pub fn parse_server_url(server: &str) -> Result<Url> {
    let candidate = if server.contains("://") {
        server.to_owned()
    } else {
        format!("http://{server}")
    };

    Url::parse(&candidate).context("server must be a valid URL")
}

pub fn ensure_absolute_url(server: &Url, maybe_url: &str) -> Result<String> {
    if Url::parse(maybe_url).is_ok() {
        return Ok(maybe_url.to_owned());
    }

    Ok(server.join(maybe_url)?.to_string())
}

pub fn validate_limit(limit: u32) -> Result<u32> {
    if (1..=100).contains(&limit) {
        Ok(limit)
    } else {
        bail!("limit must be between 1 and 100")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_default_scheme_when_missing() {
        let url = parse_server_url("localhost:3000").unwrap();
        assert_eq!(url.as_str(), "http://localhost:3000/");
    }

    #[test]
    fn preserves_explicit_scheme() {
        assert_eq!(
            parse_server_url("https://music.example.com")
                .unwrap()
                .as_str(),
            "https://music.example.com/"
        );
        assert_eq!(
            parse_server_url("http://localhost:3000").unwrap().as_str(),
            "http://localhost:3000/"
        );
    }

    #[test]
    fn parses_prerelease_versions() {
        let version = VersionInfo {
            major: 1,
            minor: 2,
            patch: 3,
            pre: Some("rc.1".to_owned()),
        };

        assert_eq!(version.semver().unwrap().to_string(), "1.2.3-rc.1");
    }
}
