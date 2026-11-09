mod files;
mod parts;
mod preview;
mod prompt;
mod search;
mod share;
mod shares;
mod shell;
mod sign_in;
mod sync_view;
mod trash;
mod update;
mod versions;

use std::future::Future;
use std::sync::Arc;

use gpui_kit::*;

use crate::desktop::Desktop;
use crate::runtime;
use shell::{Shell, SignedOut};
use sign_in::{SignIn, SignedIn};

pub fn run<V, T, F>(
    window: &Window,
    cx: &mut Context<V>,
    work: F,
    done: impl FnOnce(&mut V, T, &mut Window, &mut Context<V>) + 'static,
) where
    V: 'static,
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    let work = runtime::spawn(work);
    cx.spawn_in(window, async move |this, cx| {
        let output = work.await;
        let _ = this.update_in(cx, |view, window, cx| done(view, output, window, cx));
    })
    .detach();
}

enum Screen {
    Resuming,
    SignIn(Entity<SignIn>),
    Signed(Entity<Shell>),
}

pub struct Workspace {
    desktop: Arc<Desktop>,
    screen: Screen,
}

impl Workspace {
    pub fn new(desktop: Arc<Desktop>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut workspace = Self {
            desktop: Arc::clone(&desktop),
            screen: Screen::Resuming,
        };
        if desktop.remote().is_ok() {
            workspace.signed(None, window, cx);
        } else {
            run(
                window,
                cx,
                desktop.resume(),
                |this, resumed, window, cx| match resumed {
                    Ok(true) => this.signed(None, window, cx),
                    Ok(false) => this.signing_in(None, window, cx),
                    Err(failure) => this.signing_in(
                        Some(format!(
                            "Sign in again, the saved session did not open: {failure}"
                        )),
                        window,
                        cx,
                    ),
                },
            );
        }
        workspace
    }

    fn signing_in(&mut self, error: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let desktop = Arc::clone(&self.desktop);
        let sign_in = cx.new(|cx| SignIn::new(desktop, error, window, cx));
        cx.subscribe_in(
            &sign_in,
            window,
            |this, _, signed: &SignedIn, window, cx| {
                this.signed(signed.warning.clone(), window, cx);
            },
        )
        .detach();
        self.screen = Screen::SignIn(sign_in);
        cx.notify();
    }

    fn signed(&mut self, notice: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let desktop = Arc::clone(&self.desktop);
        let shell = cx.new(|cx| Shell::new(desktop, notice, window, cx));
        cx.subscribe_in(&shell, window, |this, _, out: &SignedOut, window, cx| {
            this.signing_in(out.reason.clone(), window, cx);
        })
        .detach();
        self.screen = Screen::Signed(shell);
        cx.notify();
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::component::ActiveTheme as _;
        let body = match &self.screen {
            Screen::Resuming => div().into_any_element(),
            Screen::SignIn(sign_in) => sign_in.clone().into_any_element(),
            Screen::Signed(shell) => shell.clone().into_any_element(),
        };
        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(body)
    }
}
