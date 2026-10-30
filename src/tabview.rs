//! A tab's WebView: its signals (URL, title, progress, popups, policy, crashes, dialogs,
//! permissions, context menu) and messages from our scripts in world `novabrowser`.
use crate::{commands, config, modes, net, web, window};
use gtk::prelude::*;
use gtk::{gio, glib};
use webkit6::prelude::*;
use webkit6::{
    ContextMenuAction, ContextMenuItem, LoadEvent, NavigationPolicyDecision, NavigationType,
    PolicyDecisionType, ResponsePolicyDecision, WebProcessTerminationReason, WebView,
};

/// Load `url` with the user agent its site wants (`content.user_agent` per site).
pub fn load(view: &WebView, url: &str) {
    set_ua_for(view, url);
    view.load_uri(url);
}

fn set_ua_for(view: &WebView, url: &str) {
    let Some(s) = WebViewExt::settings(view) else { return };
    let want = web::user_agent_for(url).unwrap_or_default();
    let have = s.user_agent().map(|u| u.to_string()).unwrap_or_default();
    // an empty UA asks WebKit for its own default
    if want.is_empty() {
        if have == web::SAFARI_UA || !have.contains("X11") {
            s.set_user_agent(None);
        }
    } else if want != have {
        s.set_user_agent(Some(&want));
    }
}

/// After a config change: settings that apply to loaded views.
pub fn config_changed() {
    let cfg = config::get();
    for v in window::all_views() {
        if let Some(s) = WebViewExt::settings(&v) {
            s.set_enable_smooth_scrolling(cfg.bool("scrolling.smooth"));
            s.set_media_playback_requires_user_gesture(!cfg.bool("content.autoplay"));
            s.set_enable_site_specific_quirks(cfg.bool("webkit.site_specific_quirks"));
        }
        if let Some(u) = v.uri() {
            set_ua_for(&v, &u);
        }
        crate::darkmode::apply(&v);
    }
}

pub fn connect(view: &WebView) {
    view.connect_uri_notify(|v| {
        if let Some(u) = v.uri() {
            window::set_url(v, u.to_string());
        }
    });
    view.connect_title_notify(|v| {
        window::set_title(v, v.title().map(|t| t.to_string()).unwrap_or_default());
    });
    view.connect_estimated_load_progress_notify(|v| {
        window::set_progress(v, v.estimated_load_progress());
    });
    view.connect_is_loading_notify(|v| window::set_loading(v, v.is_loading()));
    view.connect_is_muted_notify(window::set_audio);
    view.connect_is_playing_audio_notify(window::set_audio);
    view.connect_favicon_notify(|v| {
        let Some(t) = v.favicon() else { return };
        let png = t.save_to_png_bytes();
        let url = format!("data:image/png;base64,{}", glib::base64_encode(&png));
        window::set_favicon(v, url);
    });
    view.connect_load_changed(|v, ev| match ev {
        LoadEvent::Started => {
            crate::passwords::page_changed();
        }
        LoadEvent::Committed => {
            window::set_loading(v, true);
            crate::darkmode::apply(v);
            crate::zoom::apply(v);
        }
        LoadEvent::Finished => window::set_loading(v, false),
        _ => {}
    });
    view.connect_load_failed(|v, _ev, uri, err| {
        // HTTPS-only: a site without working https gets an error page; say why
        let cancelled = err.matches(webkit6::NetworkError::Cancelled)
            || err.matches(webkit6::PolicyError::FrameLoadInterruptedByPolicyChange);
        if !cancelled && net::was_upgraded(uri) {
            let host = net::authority(uri).map(|(a, _)| a.to_string()).unwrap_or_default();
            let text = format!("{host}: https failed ({}). HTTPS-only upgraded it; :https-toggle (<Space>th) allows http here", err.message());
            window::in_owner(v, || window::message("warning", &text));
        }
        // policy errors (downloads, blocked navigations) keep WebKit's handling
        !cancelled && err.kind::<webkit6::PolicyError>().is_none() && window::load_error(v, uri, err)
    });
    view.connect_load_failed_with_tls_errors(|v, uri, cert, errors| {
        crate::permissions::tls_error(v, uri, cert, errors);
        true
    });
    view.connect_web_process_terminated(|v, reason| {
        let why = match reason {
            WebProcessTerminationReason::ExceededMemoryLimit => "out of memory",
            WebProcessTerminationReason::TerminatedByApi => "killed",
            _ => "crashed",
        };
        window::crashed(v, why);
    });
    view.connect_is_web_process_responsive_notify(|v| {
        if v.is_web_process_responsive() {
            crate::permissions::responsive(v);
        } else {
            crate::permissions::hung(v);
        }
    });
    view.connect_close(window::did_close);
    view.connect_create(|v, action| {
        // popups without a user gesture are refused (CEF audit A7)
        if !action.is_user_gesture() {
            window::in_owner(v, || window::message("info", "Blocked a popup the page opened by itself."));
            return None;
        }
        let private = window::is_private(v);
        let new = web::page_view(Some(v), private);
        let bg = action.mouse_button() == 2;
        window::in_owner(v, || window::adopt(new.clone(), !bg));
        Some(new.upcast())
    });
    view.connect_decide_policy(|v, d, kind| decide(v, d, kind));
    view.connect_enter_fullscreen(|v| {
        window::in_owner(v, || window::page_fullscreen(true));
        false
    });
    view.connect_leave_fullscreen(|v| {
        window::in_owner(v, || window::page_fullscreen(false));
        false
    });
    view.connect_script_dialog(crate::permissions::dialog);
    view.connect_permission_request(crate::permissions::request);
    view.connect_context_menu(context_menu);
    if let Some(fc) = view.find_controller() {
        let w = view.downgrade();
        fc.connect_found_text(move |_, n| {
            if let Some(v) = w.upgrade() {
                let text = crate::commands::find_shown(n);
                window::in_owner(&v, || window::message("find", &text));
            }
        });
        let w = view.downgrade();
        fc.connect_failed_to_find_text(move |fc| {
            if let Some(v) = w.upgrade() {
                let t = fc.search_text().unwrap_or_default();
                window::in_owner(&v, || window::message("warning", &format!("Text '{t}' not found on page!")));
            }
        });
    }
}

/// Navigation policy: middle/Ctrl-click → background tab, magnet links → aria2,
/// HTTPS-only for main-frame http GETs, per-site UA, downloads for unshowable types.
fn decide(v: &WebView, d: &webkit6::PolicyDecision, kind: PolicyDecisionType) -> bool {
    match kind {
        PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
            let Some(nd) = d.downcast_ref::<NavigationPolicyDecision>() else { return false };
            let Some(action) = nd.navigation_action() else { return false };
            let Some(url) = action.request().and_then(|r| r.uri()).map(|u| u.to_string()) else {
                return false;
            };
            if url.starts_with("magnet:") {
                d.ignore();
                if !crate::downloads::magnet(url) {
                    window::in_owner(v, || window::message("error", "magnet: links need aria2 (install aria2)"));
                }
                return true;
            }
            let link = action.navigation_type() == NavigationType::LinkClicked;
            let ctrl = action.modifiers() & gtk::gdk::ModifierType::CONTROL_MASK.bits() != 0;
            let middle = action.mouse_button() == 2;
            if link && (middle || ctrl) {
                d.ignore();
                let url = net::https_upgrade(&url).unwrap_or(url);
                window::in_owner(v, || window::open(&url, false));
                return true;
            }
            if kind == PolicyDecisionType::NavigationAction && !action.is_redirect() {
                let method = action.request().and_then(|r| r.http_method()).map(|m| m.to_string());
                if matches!(method.as_deref(), None | Some("GET") | Some("HEAD")) {
                    if let Some(https) = net::https_upgrade(&url) {
                        d.ignore();
                        load(v, &https);
                        return true;
                    }
                }
                set_ua_for(v, &url);
            }
            if kind == PolicyDecisionType::NavigationAction {
                crate::adblock::on_navigate(v, &url);
                crate::darkmode::apply_for(v, &url);
            }
            false
        }
        PolicyDecisionType::Response => {
            let Some(rd) = d.downcast_ref::<ResponsePolicyDecision>() else { return false };
            if !rd.is_mime_type_supported() && rd.is_main_frame_main_resource() {
                d.download();
                return true;
            }
            false
        }
        _ => false,
    }
}

/// Our extra right-click items in front of WebKit's own.
fn context_menu(
    v: &WebView,
    menu: &webkit6::ContextMenu,
    hit: &webkit6::HitTestResult,
) -> bool {
    let weak = v.downgrade();
    let mut items: Vec<(String, String)> = Vec::new(); // (label, action payload)
    if hit.context_is_link() {
        if let Some(link) = hit.link_uri() {
            items.push(("Open link in new tab".into(), format!("tab {link}")));
            items.push(("Open link in background tab".into(), format!("bg {link}")));
            items.push(("Open link in new window".into(), format!("win {link}")));
            // WebKit's own "open in new window" would make an unparented popup
            for a in [ContextMenuAction::OpenLinkInNewWindow] {
                remove_stock(menu, a);
            }
        }
    }
    if hit.context_is_image() {
        if let Some(src) = hit.image_uri() {
            items.push(("Open image in new tab".into(), format!("tab {src}")));
            remove_stock(menu, ContextMenuAction::OpenImageInNewWindow);
        }
    }
    if hit.context_is_media() {
        if let Some(src) = hit.media_uri() {
            items.push(("Open media in new tab".into(), format!("tab {src}")));
            remove_stock(menu, ContextMenuAction::OpenVideoInNewWindow);
            remove_stock(menu, ContextMenuAction::OpenAudioInNewWindow);
        }
    }
    if hit.context_is_selection() && !hit.context_is_editable() {
        items.push(("Search for the selection".into(), "search".into()));
    }
    for (i, (label, payload)) in items.into_iter().enumerate() {
        let act = gio::SimpleAction::new(&format!("nova-ctx-{i}"), None);
        let w = weak.clone();
        act.connect_activate(move |_, _| {
            let Some(v) = w.upgrade() else { return };
            let (kind, url) = payload.split_once(' ').unwrap_or((payload.as_str(), ""));
            let url = url.to_string();
            window::in_owner(&v, || match kind {
                "tab" => window::open(&url, true),
                "bg" => window::open(&url, false),
                "win" => window::open_window(Some(url)),
                _ => {
                    let v2 = v.clone();
                    web::eval_world(&v, "JSON.stringify(getSelection().toString())", move |r| {
                        if let Some(t) = r.and_then(|r| r.as_str().map(str::to_string)) {
                            window::in_owner(&v2, || {
                                window::open(&commands::url_or_search(t.trim()), true)
                            });
                        }
                    });
                }
            });
        });
        menu.insert(&ContextMenuItem::from_gaction(&act, &label, None), i as i32);
    }
    false
}

fn remove_stock(menu: &webkit6::ContextMenu, action: ContextMenuAction) {
    let items = menu.items();
    for it in items {
        if it.stock_action() == action {
            menu.remove(&it);
        }
    }
}

/// `webkit.messageHandlers.nova.postMessage` from our world in a page.
pub fn page_message(view: &WebView, text: &str) {
    let (cmd, arg) = text.split_once(' ').unwrap_or((text, ""));
    window::in_owner(view, || {
        let active = window::active_view().is_some_and(|a| web::id(&a) == web::id(view));
        match cmd {
            "focus" if active => modes::on_focus(arg == "1"),
            "pw" => crate::passwords::on_message(view, arg),
            "adblock" => crate::adblock::on_message(view, arg),
            "media" => {
                let (code, src) = arg.split_once(' ').unwrap_or((arg, ""));
                let what = if code == "3" { "can't decode" } else { "can't play (format not supported or not found)" };
                let name = src.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("media");
                window::message(
                    "warning",
                    &format!("{what}: {name}. Missing codec? Install gst-libav and gst-plugins-bad."),
                );
            }
            _ => {}
        }
    });
}

/// `nova.send("page <token> <json>")` from one of our internal pages (via web.rs).
pub fn internal_action(view: &WebView, text: &str) {
    let msg = text
        .strip_prefix("page ")
        .and_then(|t| t.split_once(' '))
        .filter(|(tok, _)| *tok == crate::page_token())
        .and_then(|(_, j)| serde_json::from_str::<serde_json::Value>(j).ok());
    let Some(msg) = msg else { return };
    type OnPage = fn(&serde_json::Value) -> Result<(), String>;
    let (on_page, page): (OnPage, fn() -> String) = match msg["page"].as_str() {
        Some("settings") => (crate::settings_page::on_page, crate::settings_page::page),
        Some("history") => (crate::history::on_page, || crate::history::page(None)),
        Some("downloads") => (crate::downloads::on_page, crate::downloads::page),
        _ => (crate::marks::on_page, crate::marks::page),
    };
    window::in_owner(view, || match on_page(&msg) {
        Ok(()) => view.load_uri(&web::internal_url(&page())),
        Err(e) => window::message("error", &e),
    });
}
