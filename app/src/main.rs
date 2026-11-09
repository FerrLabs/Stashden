#![allow(clippy::wildcard_imports)]

mod describe;
mod desktop;
mod failure;
mod keychain;
mod location;
mod remembered;
mod runtime;
mod sync;
mod tray;
mod ui;
mod update;

use std::sync::Arc;

use futures::StreamExt as _;
use gpui_kit::*;

use desktop::Desktop;
use ui::Workspace;

struct MainWindow {
    desktop: Arc<Desktop>,
    open: Option<AnyWindowHandle>,
    tray: bool,
}

impl Global for MainWindow {}

fn options(cx: &App) -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Stashden".into()),
            ..TitlebarOptions::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1100.), px(720.)),
            cx,
        ))),
        window_min_size: Some(size(px(720.), px(480.))),
        app_id: Some("com.ferrlabs.roxycloud".to_owned()),
        ..WindowOptions::default()
    }
}

fn reveal(cx: &mut App) {
    if let Some(handle) = cx.global::<MainWindow>().open {
        let shown = handle.update(cx, |_, window, _| window.activate_window());
        if shown.is_ok() {
            return;
        }
    }
    let desktop = Arc::clone(&cx.global::<MainWindow>().desktop);
    let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
        window.on_window_should_close(cx, |_, cx| {
            let main = cx.global_mut::<MainWindow>();
            if main.tray && main.desktop.is_syncing() {
                main.open = None;
                true
            } else {
                cx.quit();
                false
            }
        });
        cx.new(|cx| Workspace::new(desktop, window, cx))
    });
    match opened {
        Ok((handle, _)) => cx.global_mut::<MainWindow>().open = Some(handle),
        Err(error) => {
            eprintln!("the Stashden window did not open: {error}");
            cx.quit();
        }
    }
}

fn follow_tray(tray: tray::Tray, cx: &mut App) {
    let tray::Tray { asked, icon } = tray;
    cx.spawn(async move |cx| {
        let _icon = icon;
        let mut asked = asked;
        while let Some(asked) = asked.next().await {
            cx.update(|cx| match asked {
                tray::Asked::Open => reveal(cx),
                tray::Asked::Quit => cx.quit(),
            });
        }
    })
    .detach();
}

fn main() {
    let desktop = Desktop::new(Desktop::config_directory());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            gpui_kit::init(cx);
            let installed = tray::install();
            if let Err(reason) = &installed {
                eprintln!("no tray icon, closing the window will quit: {reason}");
            }
            cx.set_global(MainWindow {
                desktop,
                open: None,
                tray: installed.is_ok(),
            });
            if let Ok(tray) = installed {
                follow_tray(tray, cx);
            }
            reveal(cx);
        });
}
