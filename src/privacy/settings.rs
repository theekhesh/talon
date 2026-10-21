use webkit6::prelude::*;

pub fn harden_webview(webview: &webkit6::WebView) {
    let settings = webview.settings().unwrap();
    settings.set_enable_https_only(true);
    settings.set_cookie_accept_policy(webkit6::CookieAcceptPolicy::Never);
    settings.set_enable_media_stream(false);
    settings.set_enable_geolocation(false);

    let user_content_manager = webview.user_content_manager().unwrap();
    let filter = webkit6::UserContentFilter::new(
        "talon-tracker-block",
        r#"[{"trigger":{"url-filter":"(doubleclick\\.net|google-analytics\\.com|googletagmanager\\.com|googlesyndication\\.com)"},"action":{"type":"block"}}]"#,
    );
    user_content_manager.add_filter(&filter);
}
