use std::fs;
use std::path::PathBuf;

use libadwaita as adw;
use rusqlite::Connection;

pub fn init_db(_app: &adw::Application) {
	let data_dir = std::env::var_os("XDG_DATA_HOME")
		.map(PathBuf::from)
		.or_else(|| {
			std::env::var_os("HOME")
				.map(|home| PathBuf::from(home).join(".local").join("share"))
		})
		.expect("Could not determine the user data directory");
	let app_data_dir = data_dir.join("talon");
	fs::create_dir_all(&app_data_dir).expect("Could not create Talon data directory");

	let connection = Connection::open(app_data_dir.join("talon.db"))
		.expect("Could not open Talon database");
	connection
		.execute_batch(
			"CREATE TABLE IF NOT EXISTS privacy_settings (
				key TEXT PRIMARY KEY,
				value TEXT NOT NULL
			);",
		)
		.expect("Could not initialize Talon database");
}
