use std::cell::RefCell;
use std::rc::Rc;

use webkit6::{UserContentFilter, UserContentFilterStore, UserContentManager};

use super::storage;

const FILTER_ID: &str = "talon-built-in-filter";

pub fn install_filter(
	manager: &UserContentManager,
	available_filter: Rc<RefCell<Option<UserContentFilter>>>,
) {
	let directory = match storage::data_dir() {
		Ok(directory) => directory.join("filters"),
		Err(error) => {
			eprintln!("Could not determine filter storage directory: {error}");
			return;
		}
	};
	if let Err(error) = std::fs::create_dir_all(&directory) {
		eprintln!("Could not create filter storage directory: {error}");
		return;
	}
	let Some(directory) = directory.to_str() else {
		eprintln!("Filter storage path is not valid UTF-8");
		return;
	};
	let store = UserContentFilterStore::new(directory);
	let manager = manager.clone();
	store.save(
		FILTER_ID,
		&gtk4::glib::Bytes::from_static(include_bytes!("../../data/filters.json")),
		None::<&gtk4::gio::Cancellable>,
		move |result| match result {
			Ok(filter) => {
				if storage::get_setting("ad_blocking", "true")
					.unwrap_or_else(|error| {
						eprintln!("Could not read ad-blocking setting: {error}");
						"false".to_owned()
					}) == "true"
				{
					manager.add_filter(&filter);
				}
				*available_filter.borrow_mut() = Some(filter);
			}
			Err(error) => eprintln!("Could not compile built-in content filter: {error}"),
		},
	);
}

pub fn set_enabled(
	manager: &UserContentManager,
	available_filter: &Rc<RefCell<Option<UserContentFilter>>>,
	enabled: bool,
) {
	if enabled {
		if let Some(filter) = available_filter.borrow().as_ref() {
			manager.add_filter(filter);
		}
	} else {
		manager.remove_filter_by_id(FILTER_ID);
	}
}
