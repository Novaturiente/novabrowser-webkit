// w16 spike: bare WebKitGTK view, none of novabrowser's injected scripts or settings.
// usage: [W16_NOJS=1] w16-bare <cookies.sqlite> <url>
use gtk::prelude::*;
use webkit6::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (cookies, url) = (args[1].clone(), args[2].clone());
    let app = gtk::Application::builder().application_id("dev.nova.w16bare").build();
    app.connect_activate(move |app| {
        let dir = std::path::Path::new(&cookies).parent().unwrap().to_str().unwrap().to_string();
        let session = webkit6::NetworkSession::new(Some(&dir), Some(&dir));
        session
            .cookie_manager()
            .unwrap()
            .set_persistent_storage(&cookies, webkit6::CookiePersistentStorage::Sqlite);
        let view = webkit6::WebView::builder().network_session(&session).build();
        if std::env::var_os("W16_NOJS").is_some() {
            WebViewExt::settings(&view).unwrap().set_enable_javascript(false);
        }
        view.load_uri(&url);
        let win = gtk::ApplicationWindow::builder().application(app).title("w16-bare").default_width(1600).default_height(1000).child(&view).build();
        win.present();
    });
    app.run_with_args(&[args[0].clone()]);
}
