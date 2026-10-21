use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4 as gtk;
use libadwaita as adw;
use libadwaita::prelude::AdwApplicationWindowExt;
use webkit6::prelude::*;
use webkit6::{NetworkSession, UserContentFilter, UserContentManager, WebView};

use crate::privacy;

pub struct TalonWindow {
    window: adw::ApplicationWindow,
}

#[derive(Clone)]
struct BrowserUi {
    back: gtk::Button,
    forward: gtk::Button,
    reload: gtk::Button,
    progress: gtk::ProgressBar,
    status: gtk::Label,
}

#[derive(Clone)]
struct TabContext {
    session: NetworkSession,
    default_session: NetworkSession,
    manager: UserContentManager,
    omnibox: gtk::Entry,
    browser_ui: BrowserUi,
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

        let data_dir =
            privacy::storage::data_dir().expect("Could not determine Talon data directory");
        let network_dir = data_dir.join("webkit");
        let cache_dir = data_dir.join("cache");
        std::fs::create_dir_all(&network_dir).expect("Could not create WebKit data directory");
        std::fs::create_dir_all(&cache_dir).expect("Could not create WebKit cache directory");
        let network_session = NetworkSession::new(network_dir.to_str(), cache_dir.to_str());
        network_session.set_itp_enabled(true);

        let content_manager = UserContentManager::new();
        let available_filter = Rc::new(RefCell::new(None));
        privacy::adblock::install_filter(&content_manager, available_filter.clone());
        connect_downloads(&network_session);

        let header = adw::HeaderBar::new();
        let omnibox = gtk::Entry::builder()
            .placeholder_text("Search or enter address")
            .hexpand(true)
            .activates_default(true)
            .build();
        header.set_title_widget(Some(&omnibox));

        let notebook = gtk::Notebook::builder()
            .hexpand(true)
            .vexpand(true)
            .scrollable(true)
            .show_border(false)
            .build();

        let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        navigation.set_margin_start(8);
        navigation.set_margin_end(8);
        navigation.set_margin_top(6);
        navigation.set_margin_bottom(6);

        let back = gtk::Button::with_label("Back");
        let forward = gtk::Button::with_label("Forward");
        let reload = gtk::Button::with_label("Reload");
        let home = gtk::Button::with_label("Home");
        let new_tab = gtk::Button::with_label("New tab");
        let private_tab = gtk::Button::with_label("Private");
        let bookmark = gtk::Button::with_label("Bookmark");
        let bookmarks = gtk::Button::with_label("Bookmarks");
        let history = gtk::Button::with_label("History");
        let downloads = gtk::Button::with_label("Downloads");
        let settings = gtk::Button::with_label("Settings");
        let progress = gtk::ProgressBar::new();
        progress.set_visible(false);
        let status = gtk::Label::new(None);
        status.set_halign(gtk::Align::Start);
        status.set_wrap(true);
        status.set_margin_start(8);
        status.set_margin_end(8);
        status.set_visible(false);
        let browser_ui = BrowserUi {
            back: back.clone(),
            forward: forward.clone(),
            reload: reload.clone(),
            progress: progress.clone(),
            status: status.clone(),
        };

        for button in [
            &back,
            &forward,
            &reload,
            &home,
            &new_tab,
            &private_tab,
            &bookmark,
            &bookmarks,
            &history,
            &downloads,
            &settings,
        ] {
            navigation.append(button);
        }

        let initial_url = setting("homepage", "https://duckduckgo.com");
        let tab_context = TabContext {
            session: network_session.clone(),
            default_session: network_session.clone(),
            manager: content_manager.clone(),
            omnibox: omnibox.clone(),
            browser_ui: browser_ui.clone(),
        };
        append_tab(
            &notebook,
            &tab_context,
            Some(&initial_url),
            false,
        );

        let notebook_for_address = notebook.clone();
        let address_tab_context = TabContext {
            session: network_session.clone(),
            default_session: network_session.clone(),
            manager: content_manager.clone(),
            omnibox: omnibox.clone(),
            browser_ui: browser_ui.clone(),
        };
        omnibox.connect_activate(move |entry| {
            let input = entry.text().to_string();
            if let Some(view) = current_webview(&notebook_for_address) {
                view.load_uri(&resolve_address(&input));
            } else {
                append_tab(
                    &notebook_for_address,
                    &address_tab_context,
                    Some(&resolve_address(&input)),
                    false,
                );
            }
        });

        let notebook_for_navigation = notebook.clone();
        back.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook_for_navigation) {
                if view.can_go_back() {
                    view.go_back();
                }
            }
        });
        let notebook_for_navigation = notebook.clone();
        forward.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook_for_navigation) {
                if view.can_go_forward() {
                    view.go_forward();
                }
            }
        });
        let notebook_for_navigation = notebook.clone();
        reload.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook_for_navigation) {
                if view.is_loading() {
                    view.stop_loading();
                } else {
                    view.reload();
                }
            }
        });

        let notebook_for_home = notebook.clone();
        home.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook_for_home) {
                view.load_uri(&setting("homepage", "https://duckduckgo.com"));
            }
        });

        let notebook_for_new = notebook.clone();
        let new_tab_context = TabContext {
            session: network_session.clone(),
            default_session: network_session.clone(),
            manager: content_manager.clone(),
            omnibox: omnibox.clone(),
            browser_ui: browser_ui.clone(),
        };
        new_tab.connect_clicked(move |_| {
            append_tab(
                &notebook_for_new,
                &new_tab_context,
                Some(&setting("homepage", "https://duckduckgo.com")),
                false,
            );
        });

        let notebook_for_private = notebook.clone();
        let private_tab_context = TabContext {
            session: NetworkSession::new_ephemeral(),
            default_session: network_session.clone(),
            manager: content_manager.clone(),
            omnibox: omnibox.clone(),
            browser_ui: browser_ui.clone(),
        };
        private_tab.connect_clicked(move |_| {
            append_tab(
                &notebook_for_private,
                &private_tab_context,
                Some("about:blank"),
                true,
            );
        });

        let notebook_for_bookmark = notebook.clone();
        bookmark.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook_for_bookmark) {
                if let Some(url) = view.uri() {
                    let title = view
                        .title()
                        .map(|title| title.to_string())
                        .unwrap_or_else(|| url.to_string());
                    if let Err(error) = privacy::storage::add_bookmark(&url, &title) {
                        eprintln!("Could not save bookmark: {error}");
                    }
                }
            }
        });

        let window_for_bookmarks = window.clone();
        let notebook_for_bookmarks = notebook.clone();
        bookmarks.connect_clicked(move |_| {
            show_saved_pages(
                &window_for_bookmarks,
                &notebook_for_bookmarks,
                "Bookmarks",
                privacy::storage::bookmarks(),
            );
        });
        let parent_for_history = window.clone();
        let notebook_for_history = notebook.clone();
        history.connect_clicked(move |_| {
            show_history(&parent_for_history, &notebook_for_history);
        });
        let window_for_downloads = window.clone();
        downloads.connect_clicked(move |_| {
            show_downloads(&window_for_downloads);
        });

        let notebook_for_settings = notebook.clone();
        let manager_for_settings = content_manager.clone();
        let filter_for_settings = available_filter.clone();
        let window_for_settings = window.clone();
        settings.connect_clicked(move |_| {
            show_settings(
                &window_for_settings,
                &notebook_for_settings,
                &manager_for_settings,
                &filter_for_settings,
            );
        });

        let address_for_switch = omnibox.clone();
        let ui_for_switch = browser_ui.clone();
        notebook.connect_switch_page(move |notebook, page, _| {
            ui_for_switch.status.set_visible(false);
            if let Some(view) = page.downcast_ref::<WebView>() {
                address_for_switch.set_text(view.uri().as_deref().unwrap_or(""));
            }
            update_navigation(notebook, &ui_for_switch);
        });

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&header);
        root.append(&navigation);
        root.append(&progress);
        root.append(&status);
        root.append(&notebook);
        window.set_content(Some(&root));
        install_shortcuts(
            app,
            &window,
            &notebook,
            &network_session,
            &content_manager,
            &omnibox,
            &browser_ui,
        );

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn append_tab(
    notebook: &gtk::Notebook,
    tab_context: &TabContext,
    url: Option<&str>,
    private: bool,
) -> WebView {
    let webview = WebView::builder()
        .network_session(&tab_context.session)
        .user_content_manager(&tab_context.manager)
        .hexpand(true)
        .vexpand(true)
        .build();
    privacy::settings::harden_webview(&webview);
    let page = notebook.append_page(&webview, None::<&gtk::Widget>);

    let tab_label = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::new(Some("New tab"));
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(24);
    let close = gtk::Button::with_label("×");
    close.set_has_frame(false);
    tab_label.append(&title);
    tab_label.append(&close);
    notebook.set_tab_label(&webview, Some(&tab_label));
    notebook.set_tab_reorderable(&webview, true);
    notebook.set_current_page(Some(page));
    update_navigation(notebook, &tab_context.browser_ui);

    let notebook_for_close = notebook.clone();
    let view_for_close = webview.clone();
    let session_for_close = tab_context.default_session.clone();
    let manager_for_close = tab_context.manager.clone();
    let address_for_close = tab_context.omnibox.clone();
    let ui_for_close = tab_context.browser_ui.clone();
    close.connect_clicked(move |_| {
        close_tab(
            &notebook_for_close,
            &view_for_close,
            &session_for_close,
            &manager_for_close,
            &address_for_close,
            &ui_for_close,
        );
    });

    let notebook_for_create = notebook.clone();
    let session_for_create = tab_context.session.clone();
    let default_session_for_create = tab_context.default_session.clone();
    let manager_for_create = tab_context.manager.clone();
    let address_for_create = tab_context.omnibox.clone();
    let ui_for_create = tab_context.browser_ui.clone();
    let is_private = private;
    let create_context = TabContext {
        session: session_for_create.clone(),
        default_session: default_session_for_create.clone(),
        manager: manager_for_create.clone(),
        omnibox: address_for_create.clone(),
        browser_ui: ui_for_create.clone(),
    };
    webview.connect_create(move |_, _| {
        let view = append_tab(
            &notebook_for_create,
            &create_context,
            None,
            is_private,
        );
        view.upcast()
    });

    let tab_title = title.clone();
    webview.connect_title_notify(move |view| {
        tab_title.set_text(
            view.title()
                .as_deref()
                .filter(|title| !title.is_empty())
                .unwrap_or("New tab"),
        );
    });

    let is_private = private;
    let notebook_for_history = notebook.clone();
    let address_for_load = tab_context.omnibox.clone();
    let ui_for_load = tab_context.browser_ui.clone();
    let failed_load = Rc::new(std::cell::Cell::new(false));
    let failed_load_for_changed = failed_load.clone();
    webview.connect_load_changed(move |view, event| {
        let is_active =
            current_webview(&notebook_for_history).is_some_and(|active| active == *view);
        if event == webkit6::LoadEvent::Started {
            failed_load_for_changed.set(false);
        }
        if is_active {
            match event {
                webkit6::LoadEvent::Started => {
                    ui_for_load.status.set_visible(false);
                    ui_for_load.progress.set_fraction(0.0);
                    ui_for_load.progress.set_visible(true);
                }
                webkit6::LoadEvent::Committed => {
                    ui_for_load.progress.set_fraction(0.75);
                }
                webkit6::LoadEvent::Finished => {
                    ui_for_load.progress.set_fraction(1.0);
                    ui_for_load.progress.set_visible(false);
                    if !failed_load_for_changed.get() {
                        ui_for_load.status.set_visible(false);
                    }
                }
                _ => {}
            }
            ui_for_load
                .reload
                .set_label(if view.is_loading() { "Stop" } else { "Reload" });
        }
        if event == webkit6::LoadEvent::Finished {
            if notebook_for_history
                .current_page()
                .and_then(|page| notebook_for_history.nth_page(Some(page)))
                .is_some_and(|page| page == *view)
            {
                address_for_load.set_text(view.uri().as_deref().unwrap_or(""));
            }
            let (Some(url), Some(title)) = (view.uri(), view.title()) else {
                return;
            };
            if !is_private && (url.starts_with("http://") || url.starts_with("https://")) {
                if let Err(error) = privacy::storage::add_history(&url, &title) {
                    eprintln!("Could not record browsing history: {error}");
                }
            }
        }
        update_navigation(&notebook_for_history, &ui_for_load);
    });

    let ui_for_progress = tab_context.browser_ui.clone();
    let notebook_for_progress = notebook.clone();
    webview.connect_estimated_load_progress_notify(move |view| {
        if view.is_loading()
            && current_webview(&notebook_for_progress).is_some_and(|active| active == *view)
        {
            ui_for_progress
                .progress
                .set_fraction(view.estimated_load_progress());
        }
    });

    let ui_for_loading = tab_context.browser_ui.clone();
    let notebook_for_loading = notebook.clone();
    webview.connect_is_loading_notify(move |view| {
        ui_for_loading
            .reload
            .set_label(if view.is_loading() { "Stop" } else { "Reload" });
        update_navigation(&notebook_for_loading, &ui_for_loading);
    });

    let address_for_uri = tab_context.omnibox.clone();
    let notebook_for_uri = notebook.clone();
    webview.connect_uri_notify(move |view| {
        if !address_for_uri.has_focus()
            && current_webview(&notebook_for_uri).is_some_and(|current| current == *view)
        {
            address_for_uri.set_text(view.uri().as_deref().unwrap_or(""));
        }
    });

    let status_for_error = tab_context.browser_ui.status.clone();
    let notebook_for_error = notebook.clone();
    webview.connect_load_failed(move |view, _, failing_uri, error| {
        failed_load.set(true);
        if current_webview(&notebook_for_error).is_some_and(|active| active == *view) {
            status_for_error.set_text(&format!(
                "Could not load {failing_uri}: {}",
                error.message()
            ));
            status_for_error.set_visible(true);
        }
        false
    });

    if let Some(url) = url {
        webview.load_uri(url);
    }
    webview
}

fn close_tab(
    notebook: &gtk::Notebook,
    view: &WebView,
    default_session: &NetworkSession,
    manager: &UserContentManager,
    omnibox: &gtk::Entry,
    browser_ui: &BrowserUi,
) {
    if let Some(page) = notebook.page_num(view) {
        notebook.remove_page(Some(page));
        if notebook.n_pages() == 0 {
            append_tab(
                notebook,
                &TabContext {
                    session: default_session.clone(),
                    default_session: default_session.clone(),
                    manager: manager.clone(),
                    omnibox: omnibox.clone(),
                    browser_ui: browser_ui.clone(),
                },
                Some(&setting("homepage", "https://duckduckgo.com")),
                false,
            );
        } else {
            update_navigation(notebook, browser_ui);
        }
    }
}

fn update_navigation(notebook: &gtk::Notebook, browser_ui: &BrowserUi) {
    if let Some(view) = current_webview(notebook) {
        browser_ui.back.set_sensitive(view.can_go_back());
        browser_ui.forward.set_sensitive(view.can_go_forward());
        browser_ui.reload.set_sensitive(true);
        browser_ui.progress.set_visible(view.is_loading());
        if view.is_loading() {
            browser_ui
                .progress
                .set_fraction(view.estimated_load_progress());
        }
        browser_ui
            .reload
            .set_label(if view.is_loading() { "Stop" } else { "Reload" });
    } else {
        browser_ui.back.set_sensitive(false);
        browser_ui.forward.set_sensitive(false);
        browser_ui.reload.set_sensitive(false);
        browser_ui.progress.set_visible(false);
    }
}

fn install_shortcuts(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    notebook: &gtk::Notebook,
    session: &NetworkSession,
    manager: &UserContentManager,
    omnibox: &gtk::Entry,
    browser_ui: &BrowserUi,
) {
    let notebook_for_new = notebook.clone();
    let new_tab_context = TabContext {
        session: session.clone(),
        default_session: session.clone(),
        manager: manager.clone(),
        omnibox: omnibox.clone(),
        browser_ui: browser_ui.clone(),
    };
    add_window_action(app, window, "new-tab", &["<Primary>t"], move || {
        append_tab(
            &notebook_for_new,
            &new_tab_context,
            Some(&setting("homepage", "https://duckduckgo.com")),
            false,
        );
    });

    let notebook_for_close = notebook.clone();
    let session_for_close = session.clone();
    let manager_for_close = manager.clone();
    let omnibox_for_close = omnibox.clone();
    let ui_for_close = browser_ui.clone();
    add_window_action(app, window, "close-tab", &["<Primary>w"], move || {
        if let Some(view) = current_webview(&notebook_for_close) {
            close_tab(
                &notebook_for_close,
                &view,
                &session_for_close,
                &manager_for_close,
                &omnibox_for_close,
                &ui_for_close,
            );
        }
    });

    let omnibox_for_focus = omnibox.clone();
    add_window_action(app, window, "focus-address", &["<Primary>l"], move || {
        omnibox_for_focus.grab_focus();
        omnibox_for_focus.select_region(0, -1);
    });

    let notebook_for_reload = notebook.clone();
    add_window_action(app, window, "reload", &["<Primary>r"], move || {
        if let Some(view) = current_webview(&notebook_for_reload) {
            if view.is_loading() {
                view.stop_loading();
            } else {
                view.reload();
            }
        }
    });

    let notebook_for_back = notebook.clone();
    add_window_action(app, window, "go-back", &["<Alt>Left"], move || {
        if let Some(view) = current_webview(&notebook_for_back) {
            if view.can_go_back() {
                view.go_back();
            }
        }
    });
    let notebook_for_forward = notebook.clone();
    add_window_action(app, window, "go-forward", &["<Alt>Right"], move || {
        if let Some(view) = current_webview(&notebook_for_forward) {
            if view.can_go_forward() {
                view.go_forward();
            }
        }
    });

    let notebook_for_next = notebook.clone();
    add_window_action(app, window, "next-tab", &["<Primary>Tab"], move || {
        switch_tab(&notebook_for_next, 1);
    });
    let notebook_for_previous = notebook.clone();
    add_window_action(
        app,
        window,
        "previous-tab",
        &["<Primary><Shift>Tab"],
        move || switch_tab(&notebook_for_previous, -1),
    );

    let app_for_window = app.clone();
    add_window_action(app, window, "new-window", &["<Primary>n"], move || {
        TalonWindow::new(&app_for_window).present();
    });
}

fn add_window_action(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    name: &str,
    accelerators: &[&str],
    callback: impl Fn() + 'static,
) {
    let action = gtk::gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| callback());
    window.add_action(&action);
    app.set_accels_for_action(&format!("win.{name}"), accelerators);
}

fn switch_tab(notebook: &gtk::Notebook, direction: i32) {
    let count = notebook.n_pages();
    if count < 2 {
        return;
    }
    let current = notebook.current_page().unwrap_or(0) as i32;
    let next = (current + direction).rem_euclid(count as i32);
    notebook.set_current_page(Some(next as u32));
}

fn current_webview(notebook: &gtk::Notebook) -> Option<WebView> {
    let page = notebook.current_page()?;
    notebook.nth_page(Some(page))?.downcast::<WebView>().ok()
}

fn resolve_address(input: &str) -> String {
    resolve_address_with_engine(
        input,
        &setting("search_engine", "https://duckduckgo.com/?q="),
    )
}

fn resolve_address_with_engine(input: &str, engine: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return setting("homepage", "https://duckduckgo.com");
    }
    if let Some((scheme, remainder)) = input.split_once("://") {
        if scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http") {
            return format!("{}://{remainder}", scheme.to_ascii_lowercase());
        }
    }
    if input.eq_ignore_ascii_case("about:blank") {
        return "about:blank".to_owned();
    }
    let has_whitespace = input.chars().any(char::is_whitespace);
    let local_address =
        input == "localhost" || input.starts_with("localhost:") || input.starts_with("localhost/");
    let ipv6_literal = input.starts_with('[') && input.contains(']');
    if !has_whitespace && (local_address || ipv6_literal || input.contains('.')) {
        return format!("https://{input}");
    }
    let encoded = percent_encode(input);
    if engine.contains("%s") {
        engine.replace("%s", &encoded)
    } else {
        format!("{engine}{encoded}")
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn setting(key: &str, default: &str) -> String {
    match privacy::storage::get_setting(key, default) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Could not read {key} preference: {error}");
            default.to_owned()
        }
    }
}

fn show_saved_pages(
    parent: &adw::ApplicationWindow,
    notebook: &gtk::Notebook,
    title: &str,
    pages: Result<Vec<(String, String)>, Box<dyn std::error::Error>>,
) {
    let pages = match pages {
        Ok(pages) => pages,
        Err(error) => {
            eprintln!("Could not load {title}: {error}");
            return;
        }
    };
    let window = utility_window(parent, title, 540, 500);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    for (page_title, url) in pages {
        let button = gtk::Button::with_label(&format!("{page_title}\n{url}"));
        button.set_hexpand(true);
        button.set_halign(gtk::Align::Fill);
        let notebook = notebook.clone();
        let window = window.clone();
        button.connect_clicked(move |_| {
            if let Some(view) = current_webview(&notebook) {
                view.load_uri(&url);
            }
            window.close();
        });
        list.append(&button);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .build();
    window.set_child(Some(&scroll));
    window.present();
}

fn show_history(parent: &adw::ApplicationWindow, notebook: &gtk::Notebook) {
    let window = utility_window(parent, "History", 600, 520);
    let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);
    let clear = gtk::Button::with_label("Clear browsing history");
    let window_for_clear = window.clone();
    let parent_for_clear = parent.clone();
    let notebook_for_clear = notebook.clone();
    clear.connect_clicked(move |_| match privacy::storage::clear_history() {
        Ok(()) => {
            window_for_clear.close();
            show_history(&parent_for_clear, &notebook_for_clear);
        }
        Err(error) => eprintln!("Could not clear browsing history: {error}"),
    });
    container.append(&clear);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    match privacy::storage::history() {
        Ok(pages) => {
            for (title, url) in pages {
                let button = gtk::Button::with_label(&format!("{title}\n{url}"));
                button.set_hexpand(true);
                button.set_halign(gtk::Align::Fill);
                let notebook = notebook.clone();
                let window = window.clone();
                button.connect_clicked(move |_| {
                    if let Some(view) = current_webview(&notebook) {
                        view.load_uri(&url);
                    }
                    window.close();
                });
                list.append(&button);
            }
        }
        Err(error) => eprintln!("Could not load browsing history: {error}"),
    }
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .build();
    container.append(&scroll);
    window.set_child(Some(&container));
    window.present();
}

fn show_settings(
    parent: &adw::ApplicationWindow,
    notebook: &gtk::Notebook,
    manager: &UserContentManager,
    available_filter: &Rc<RefCell<Option<UserContentFilter>>>,
) {
    let window = utility_window(parent, "Settings", 540, 500);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let homepage_label = gtk::Label::new(Some("Homepage"));
    homepage_label.set_halign(gtk::Align::Start);
    let homepage = gtk::Entry::builder()
        .text(setting("homepage", "https://duckduckgo.com"))
        .placeholder_text("https://example.com")
        .build();
    homepage.connect_activate(|entry| {
        if let Err(error) = privacy::storage::set_setting("homepage", entry.text().as_str()) {
            eprintln!("Could not save homepage preference: {error}");
        }
    });

    let javascript = preference_toggle("Enable JavaScript", "javascript", true);
    let media = preference_toggle("Enable audio, video and camera access", "media", true);
    let blocking = preference_toggle("Block common ads and trackers", "ad_blocking", true);
    let notebook_for_javascript = notebook.clone();
    javascript.connect_toggled(move |button| {
        apply_web_preference(&notebook_for_javascript, "javascript", button.is_active());
    });
    let notebook_for_media = notebook.clone();
    media.connect_toggled(move |button| {
        apply_web_preference(&notebook_for_media, "media", button.is_active());
    });

    let save = gtk::Button::with_label("Save settings");
    let homepage_for_save = homepage.clone();
    let window_for_save = window.clone();
    save.connect_clicked(move |_| {
        let value = homepage_for_save.text().trim().to_string();
        if !value.is_empty() {
            if let Err(error) = privacy::storage::set_setting("homepage", &value) {
                eprintln!("Could not save homepage preference: {error}");
                return;
            }
        }
        window_for_save.close();
    });

    let manager = manager.clone();
    let filter = available_filter.clone();
    blocking.connect_toggled(move |button| {
        let enabled = button.is_active();
        if let Err(error) =
            privacy::storage::set_setting("ad_blocking", if enabled { "true" } else { "false" })
        {
            eprintln!("Could not save ad-blocking preference: {error}");
            return;
        }
        privacy::adblock::set_enabled(&manager, &filter, enabled);
    });

    content.append(&homepage_label);
    content.append(&homepage);
    content.append(&javascript);
    content.append(&media);
    content.append(&blocking);
    content.append(&save);
    window.set_child(Some(&content));
    window.present();
}

fn apply_web_preference(notebook: &gtk::Notebook, key: &str, enabled: bool) {
    for index in 0..notebook.n_pages() {
        if let Some(view) = notebook
            .nth_page(Some(index))
            .and_then(|widget| widget.downcast::<WebView>().ok())
        {
            if let Some(settings) = webkit6::prelude::WebViewExt::settings(&view) {
                match key {
                    "javascript" => settings.set_enable_javascript(enabled),
                    "media" => {
                        settings.set_enable_media(enabled);
                        settings.set_enable_media_stream(enabled);
                        settings.set_enable_webrtc(enabled);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn preference_toggle(label: &str, key: &str, default: bool) -> gtk::CheckButton {
    let toggle = gtk::CheckButton::with_label(label);
    toggle.set_active(setting(key, if default { "true" } else { "false" }) == "true");
    let key = key.to_owned();
    toggle.connect_toggled(move |button| {
        if let Err(error) =
            privacy::storage::set_setting(&key, if button.is_active() { "true" } else { "false" })
        {
            eprintln!("Could not save preference {key}: {error}");
        }
    });
    toggle
}

fn show_downloads(parent: &adw::ApplicationWindow) {
    let downloads_dir = download_directory();
    let window = utility_window(parent, "Downloads", 500, 180);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.append(&gtk::Label::new(Some(&format!(
        "Downloads are saved to:\n{}",
        downloads_dir.display()
    ))));
    content.append(&gtk::Label::new(Some(
        "Use your file manager to open or manage downloaded files.",
    )));
    window.set_child(Some(&content));
    window.present();
}

fn utility_window(
    parent: &adw::ApplicationWindow,
    title: &str,
    width: i32,
    height: i32,
) -> gtk::Window {
    gtk::Window::builder()
        .transient_for(parent)
        .title(title)
        .default_width(width)
        .default_height(height)
        .modal(false)
        .build()
}

fn download_directory() -> PathBuf {
    let path = std::env::var_os("XDG_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads")))
        .unwrap_or_else(|| std::env::temp_dir().join("talon-downloads"));
    if let Err(error) = std::fs::create_dir_all(&path) {
        eprintln!("Could not create downloads directory: {error}");
    }
    path
}

fn connect_downloads(session: &NetworkSession) {
    session.connect_download_started(|_, download| {
        let directory = download_directory();
        download.connect_decide_destination(move |download, suggested| {
            let filename = PathBuf::from(suggested)
                .file_name()
                .map(|name| name.to_os_string())
                .unwrap_or_else(|| "download".into());
            let destination = directory.join(filename);
            let file = gtk::gio::File::for_path(destination);
            let uri = file.uri();
            download.set_destination(uri.as_str());
            true
        });
        download.connect_failed(|_, error| eprintln!("Download failed: {error}"));
    });
}

#[cfg(test)]
mod tests {
    use super::resolve_address_with_engine;

    #[test]
    fn resolves_web_addresses_and_localhost() {
        assert_eq!(
            resolve_address_with_engine("HTTP://example.com/path", "https://search.test/?q="),
            "http://example.com/path"
        );
        assert_eq!(
            resolve_address_with_engine("localhost:3000", "https://search.test/?q="),
            "https://localhost:3000"
        );
        assert_eq!(
            resolve_address_with_engine("about:blank", "https://search.test/?q="),
            "about:blank"
        );
    }

    #[test]
    fn encodes_search_queries_and_supports_templates() {
        assert_eq!(
            resolve_address_with_engine("rust gtk tutorial", "https://search.test/?q="),
            "https://search.test/?q=rust%20gtk%20tutorial"
        );
        assert_eq!(
            resolve_address_with_engine("rust & gtk", "https://search.test/?q=%s"),
            "https://search.test/?q=rust%20%26%20gtk"
        );
    }
}
