use gtk4 as gtk;
use libadwaita::prelude::AdwApplicationWindowExt;
use libadwaita as adw;
use webkit6::prelude::*;
use webkit6::WebView;

use crate::privacy;

pub struct TalonWindow {
    window: adw::ApplicationWindow,
    notebook: gtk::Notebook,
}

impl TalonWindow {
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Talon")
            .default_width(1200)
            .default_height(800)
            .build();

        window.set_icon_name(Some("org.talon.Browser"));

        let header = adw::HeaderBar::new();
        let omnibox = gtk::Entry::builder()
            .placeholder_text("Search or enter address")
            .hexpand(true)
            .build();
        header.pack_start(&omnibox);

        let notebook = gtk::Notebook::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let (initial_view, initial_tab_label) = Self::create_web_tab("https://example.com");
        notebook.append_page(&initial_view, Some(&initial_tab_label));

        let notebook_clone = notebook.clone();
        omnibox.connect_activate(move |entry| {
            let url = entry.text();
            if let Some(current_page) = notebook_clone.current_page() {
                if let Some(child) = notebook_clone.nth_page(Some(current_page)) {
                    if let Some(webview) = child.downcast_ref::<WebView>() {
                        webview.load_uri(&url);
                    }
                }
            }
        });

        privacy::settings::harden_webview(&initial_view);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.append(&header);
        vbox.append(&notebook);
        window.set_content(Some(&vbox));

        Self { window, notebook }
    }

    fn create_web_tab(url: &str) -> (WebView, gtk::Label) {
        let webview = WebView::new();
        webview.load_uri(url);
        let label = gtk::Label::new(Some("Talon - New Tab"));
        (webview, label)
    }

    pub fn present(&self) {
        let _ = &self.notebook;
        self.window.present();
    }
}
