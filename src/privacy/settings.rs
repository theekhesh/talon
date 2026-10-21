use super::storage;

pub fn harden_webview(webview: &webkit6::WebView) {
	let Some(settings) = webkit6::prelude::WebViewExt::settings(webview) else {
		eprintln!("Could not access WebKit settings");
		return;
	};
	settings.set_enable_javascript(read_bool("javascript", true));
	settings.set_enable_media(read_bool("media", true));
	settings.set_enable_media_stream(read_bool("media", true));
	settings.set_enable_webrtc(read_bool("media", true));
	settings.set_enable_webgl(true);
	settings.set_enable_hyperlink_auditing(false);
	settings.set_enable_dns_prefetching(false);
	settings.set_media_playback_requires_user_gesture(true);
}

fn read_bool(key: &str, default: bool) -> bool {
	match storage::get_setting(key, if default { "true" } else { "false" }) {
		Ok(value) => value == "true",
		Err(error) => {
			eprintln!("Could not read {key} preference: {error}");
			default
		}
	}
}
