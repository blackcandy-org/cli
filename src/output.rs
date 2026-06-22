use crate::api::{Album, Artist, Playlist, SearchResponse, Song};

pub fn songs(songs: &[Song]) {
    if songs.is_empty() {
        println!("No songs found.");
        return;
    }

    print_rows(
        &["ID", "Title", "Artist", "Album", "Time", "Fmt"],
        songs.iter().map(|song| {
            vec![
                song.id.to_string(),
                song.name.clone(),
                song.artist_name.clone(),
                song.album_name.clone(),
                format_duration(song.duration),
                song.format.clone().unwrap_or_default(),
            ]
        }),
    );
}

pub fn playlists(playlists: &[Playlist]) {
    if playlists.is_empty() {
        println!("No playlists found.");
        return;
    }

    print_rows(
        &["ID", "Name", "Kind"],
        playlists.iter().map(|playlist| {
            vec![
                playlist.id.to_string(),
                playlist.name.clone(),
                if playlist.is_favorite {
                    "favorite".to_owned()
                } else {
                    "playlist".to_owned()
                },
            ]
        }),
    );
}

pub fn search(response: &SearchResponse) {
    section("Songs");
    songs(&response.songs);
    section("Albums");
    albums(&response.albums);
    section("Artists");
    artists(&response.artists);
    section("Playlists");
    playlists(&response.playlists);
}

fn albums(albums: &[Album]) {
    if albums.is_empty() {
        println!("No albums found.");
        return;
    }

    print_rows(
        &["ID", "Album", "Artist", "Year", "Genre"],
        albums.iter().map(|album| {
            vec![
                album.id.to_string(),
                album.name.clone(),
                album.artist_name.clone(),
                album.year.map(|year| year.to_string()).unwrap_or_default(),
                album.genre.clone().unwrap_or_default(),
            ]
        }),
    );
}

fn artists(artists: &[Artist]) {
    if artists.is_empty() {
        println!("No artists found.");
        return;
    }

    print_rows(
        &["ID", "Artist", "Kind"],
        artists.iter().map(|artist| {
            vec![
                artist.id.to_string(),
                artist.name.clone(),
                if artist.is_various {
                    "various".to_owned()
                } else {
                    String::new()
                },
            ]
        }),
    );
}

fn section(title: &str) {
    println!();
    println!("{title}");
}

fn print_rows<I>(headers: &[&str], rows: I)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let rows: Vec<Vec<String>> = rows.into_iter().collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();

    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    print_row(headers.iter().map(|value| value.to_string()), &widths);
    print_row(widths.iter().map(|width| "-".repeat(*width)), &widths);

    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row<I>(cells: I, widths: &[usize])
where
    I: IntoIterator<Item = String>,
{
    let mut cells = cells.into_iter().enumerate().peekable();
    while let Some((index, cell)) = cells.next() {
        let width = widths[index];
        if cells.peek().is_some() {
            print!("{cell:width$}  ");
        } else {
            print!("{cell}");
        }
    }
    println!();
}

fn format_duration(duration: Option<f64>) -> String {
    let Some(duration) = duration else {
        return String::new();
    };

    let seconds = duration.round() as u64;
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{minutes}:{seconds:02}")
}
