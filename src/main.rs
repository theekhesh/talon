use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;

mod privacy;
mod ui;

fn main() {
    // WebKit's sandbox cannot start in Codespaces without unprivileged user namespaces.
    // Never ship this setting; Flatpak provides the production sandbox.
    if std::env::var("CODESPACES").is_ok() {
        std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
    }

    let app = adw::Application::builder()
        .application_id("org.talon.Browser")
        .build();

    app.connect_startup(|_| {
        adw::init().expect("Failed to initialize Libadwaita");
    });

    app.connect_activate(|app| {
        privacy::storage::init_db().expect("Failed to initialize Talon database");

        let window = ui::window::TalonWindow::new(app);
        window.present();
    });

    app.run();
}
