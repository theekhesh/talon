use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

pub fn data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
	let data_dir = std::env::var_os("XDG_DATA_HOME")
		.map(PathBuf::from)
		.or_else(|| {
			std::env::var_os("HOME")
				.map(|home| PathBuf::from(home).join(".local").join("share"))
		})
		.ok_or("Could not determine the user data directory")?;
	Ok(data_dir.join("talon"))
}

fn connection() -> Result<Connection, Box<dyn std::error::Error>> {
	let directory = data_dir()?;
	fs::create_dir_all(&directory)?;
	Ok(Connection::open(directory.join("talon.db"))?)
}

pub fn init_db() -> Result<(), Box<dyn std::error::Error>> {
	connection()?.execute_batch(
		"CREATE TABLE IF NOT EXISTS privacy_settings (
			key TEXT PRIMARY KEY,
			value TEXT NOT NULL
		);
		CREATE TABLE IF NOT EXISTS bookmarks (
			url TEXT PRIMARY KEY,
			title TEXT NOT NULL,
			folder TEXT NOT NULL DEFAULT 'Bookmarks',
			created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
		);
		CREATE TABLE IF NOT EXISTS history (
			id INTEGER PRIMARY KEY,
			url TEXT NOT NULL,
			title TEXT NOT NULL,
			visited_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
		);
		CREATE INDEX IF NOT EXISTS history_visited_at ON history(visited_at DESC);",
	)?;
	Ok(())
}

pub fn get_setting(key: &str, default: &str) -> Result<String, Box<dyn std::error::Error>> {
	let connection = connection()?;
	let value = connection.query_row(
		"SELECT value FROM privacy_settings WHERE key = ?1",
		[key],
		|row| row.get(0),
	);
	match value {
		Ok(value) => Ok(value),
		Err(rusqlite::Error::QueryReturnedNoRows) => Ok(default.to_owned()),
		Err(error) => Err(error.into()),
	}
}

pub fn set_setting(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
	connection()?.execute(
		"INSERT INTO privacy_settings(key, value) VALUES (?1, ?2)
		 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
		[key, value],
	)?;
	Ok(())
}

pub fn add_bookmark(url: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
	connection()?.execute(
		"INSERT INTO bookmarks(url, title) VALUES (?1, ?2)
		 ON CONFLICT(url) DO UPDATE SET title = excluded.title",
		[url, title],
	)?;
	Ok(())
}

pub fn bookmarks() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
	let connection = connection()?;
	let mut statement =
		connection.prepare("SELECT title, url FROM bookmarks ORDER BY title COLLATE NOCASE")?;
	let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
	Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn add_history(url: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
	connection()?.execute(
		"INSERT INTO history(url, title) VALUES (?1, ?2)",
		[url, title],
	)?;
	Ok(())
}

pub fn history() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
	let connection = connection()?;
	let mut statement =
		connection.prepare("SELECT title, url FROM history ORDER BY id DESC LIMIT 1000")?;
	let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
	Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
	connection()?.execute("DELETE FROM history", [])?;
	Ok(())
}
