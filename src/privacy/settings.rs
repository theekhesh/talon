pub fn harden_webview(webview: &webkit6::WebView) {
    let settings = webkit6::prelude::WebViewExt::settings(webview).unwrap();
    settings.set_enable_media(false);
    settings.set_enable_media_stream(false);
    settings.set_enable_webrtc(false);
    settings.set_enable_webgl(false);
}
