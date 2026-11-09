use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use stashden_client::Methods;

use super::parts;
use super::run;
use crate::describe::server_address;
use crate::desktop::Desktop;

pub struct SignedIn {
    pub warning: Option<String>,
}

pub struct SignIn {
    desktop: Arc<Desktop>,
    server: Entity<InputState>,
    email: Entity<InputState>,
    password: Entity<InputState>,
    methods: Option<Methods>,
    busy: bool,
    error: Option<String>,
}

impl EventEmitter<SignedIn> for SignIn {}

impl SignIn {
    pub fn new(
        desktop: Arc<Desktop>,
        error: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let last = desktop.last_server().unwrap_or_default();
        let server = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("https://stashden.example.com")
                .default_value(last.clone())
        });
        let email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
        let password = cx.new(|cx| InputState::new(window, cx).masked(true));

        cx.subscribe_in(
            &server,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.load_methods(window, cx);
                }
            },
        )
        .detach();
        cx.subscribe_in(
            &password,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.sign_in(window, cx);
                }
            },
        )
        .detach();

        let mut sign_in = Self {
            desktop,
            server,
            email,
            password,
            methods: None,
            busy: false,
            error,
        };
        if !last.is_empty() {
            sign_in.load_methods(window, cx);
        }
        sign_in
    }

    fn address(&self, cx: &App) -> Option<String> {
        let typed = self.server.read(cx).value();
        (!typed.trim().is_empty()).then(|| server_address(&typed))
    }

    fn load_methods(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(server) = self.address(cx) else {
            return;
        };
        run(
            window,
            cx,
            Desktop::methods(server),
            |this, methods, _, cx| {
                this.methods = methods.ok();
                cx.notify();
            },
        );
    }

    fn missing_server(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let server = self.address(cx);
        if server.is_none() {
            self.error = Some("Type the address of your Stashden server first.".to_owned());
            cx.notify();
        }
        server
    }

    fn sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(server) = self.missing_server(cx) else {
            return;
        };
        let email = self.email.read(cx).value().trim().to_owned();
        let password = self.password.read(cx).value().to_string();
        let work = Arc::clone(&self.desktop).sign_in(server, email, password);
        self.begin(work, window, cx);
    }

    fn sign_in_with_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(server) = self.missing_server(cx) else {
            return;
        };
        let work = Arc::clone(&self.desktop).sign_in_with_provider(server);
        self.begin(work, window, cx);
    }

    fn begin(
        &mut self,
        work: impl Future<Output = Result<Option<String>, crate::failure::Failure>> + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        self.error = None;
        cx.notify();
        run(window, cx, work, |this, outcome, _, cx| {
            this.busy = false;
            match outcome {
                Ok(warning) => cx.emit(SignedIn { warning }),
                Err(failure) => this.error = Some(failure.to_string()),
            }
            cx.notify();
        });
    }

    fn forgot_password(&mut self, cx: &mut Context<Self>) {
        let Some(server) = self.missing_server(cx) else {
            return;
        };
        if let Err(failure) = Desktop::forgot_password(&server) {
            self.error = Some(failure.to_string());
            cx.notify();
        }
    }
}

impl Render for SignIn {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let password = self.methods.is_none_or(|methods| methods.password);
        let reset = self.methods.is_some_and(|methods| methods.password_reset);
        let oidc = self.methods.is_some_and(|methods| methods.oidc);

        let field = |label: &'static str, input: &Entity<InputState>| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(label))
                .child(Input::new(input))
        };

        div().size_full().flex().justify_center().pt_20().child(
            div()
                .w(px(380.))
                .flex()
                .flex_col()
                .gap_4()
                .child(parts::heading("Sign in to Stashden"))
                .children(self.error.clone().map(|error| parts::failure(error, cx)))
                .child(field("Server", &self.server))
                .when(password, |form| {
                    form.child(field("Email", &self.email))
                        .child(field("Password", &self.password))
                        .child(
                            Button::new("sign-in")
                                .primary()
                                .label(if self.busy { "Signing in" } else { "Sign in" })
                                .disabled(self.busy)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.sign_in(window, cx)),
                                ),
                        )
                        .when(reset, |form| {
                            form.child(
                                Button::new("forgot")
                                    .link()
                                    .label("Forgot your password?")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.forgot_password(cx)),
                                    ),
                            )
                        })
                })
                .when(oidc, |form| {
                    let provider = Button::new("provider")
                        .label("Sign in with your identity provider")
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.sign_in_with_provider(window, cx);
                        }));
                    form.child(if password {
                        provider
                    } else {
                        provider.primary()
                    })
                }),
        )
    }
}
