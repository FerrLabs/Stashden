use std::sync::Arc;

use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use stashden_core::node::{Hit, NodeKind};

use super::parts;
use super::run;
use crate::describe;
use crate::desktop::Desktop;
use crate::failure::Failure;
use crate::location::{Folder, parent_folder_of};

pub struct Chosen(pub Folder);

pub struct SearchPanel {
    desktop: Arc<Desktop>,
    term: Entity<InputState>,
    busy: bool,
    searched: Option<String>,
    hits: Vec<Hit>,
    failure: Option<String>,
}

impl EventEmitter<Chosen> for SearchPanel {}

pub fn destination(hit: &Hit) -> Folder {
    match hit.node.kind {
        NodeKind::Directory => Folder::new(&hit.path),
        NodeKind::File => parent_folder_of(&hit.path),
    }
}

pub fn summary(hit: &Hit) -> String {
    let what = match hit.node.kind {
        NodeKind::Directory => "Folder".to_owned(),
        NodeKind::File => describe::size(hit.node.size),
    };
    let parent = parent_folder_of(&hit.path);
    let place = if parent.is_root() {
        "Files".to_owned()
    } else {
        parent.as_str().to_owned()
    };
    format!("{what}, in {place}")
}

impl SearchPanel {
    pub fn new(desktop: Arc<Desktop>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let term =
            cx.new(|cx| InputState::new(window, cx).placeholder("Part of a file or folder name"));
        term.focus_handle(cx).focus(window, cx);
        cx.subscribe_in(&term, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.search(window, cx);
            }
        })
        .detach();
        Self {
            desktop,
            term,
            busy: false,
            searched: None,
            hits: Vec::new(),
            failure: None,
        }
    }

    fn search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let term = self.term.read(cx).value().trim().to_owned();
        if term.is_empty() {
            return;
        }
        self.busy = true;
        self.failure = None;
        cx.notify();
        let remote = self.desktop.remote();
        let query = term.clone();
        let work = async move { Ok::<_, Failure>(remote?.search(&query).await?) };
        run(window, cx, work, move |this, found, _, cx| {
            this.busy = false;
            match found {
                Ok(hits) => {
                    this.hits = hits;
                    this.searched = Some(term);
                }
                Err(failure) => this.failure = Some(format!("Searching failed: {failure}")),
            }
            cx.notify();
        });
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let results = match &self.searched {
            _ if self.busy => Some(parts::state("Searching", cx).into_any_element()),
            Some(term) if self.hits.is_empty() => Some(
                parts::state(format!("Nothing is named like \"{term}\"."), cx).into_any_element(),
            ),
            Some(_) => Some(
                div()
                    .flex()
                    .flex_col()
                    .children(self.hits.iter().enumerate().map(|(index, hit)| {
                        let folder = destination(hit);
                        parts::row(cx)
                            .id(("hit", index))
                            .cursor_pointer()
                            .on_click(
                                cx.listener(move |_, _, _, cx| cx.emit(Chosen(folder.clone()))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(hit.node.name.clone())
                                    .child(parts::hint(summary(hit), cx)),
                            )
                    }))
                    .into_any_element(),
            ),
            None => None,
        };
        panel("Search", cx)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.term)))
                    .child(
                        Button::new("search-go")
                            .primary()
                            .label("Search")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.search(window, cx))),
                    ),
            )
            .children(
                self.failure
                    .clone()
                    .map(|failure| parts::failure(failure, cx)),
            )
            .children(results)
    }
}

pub fn panel(title: &'static str, cx: &App) -> Div {
    use gpui_kit::component::ActiveTheme as _;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(parts::heading(title))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use stashden_core::node::{Hit, Node, NodeKind};
    use uuid::Uuid;

    use super::{destination, summary};
    use crate::location::Folder;

    fn hit(path: &str, kind: NodeKind, size: i64) -> Hit {
        Hit {
            node: Node {
                id: Uuid::now_v7(),
                owner_id: Uuid::now_v7(),
                parent_id: None,
                name: path.rsplit('/').next().unwrap().to_owned(),
                kind,
                blob_hash: None,
                size,
                etag: String::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deleted_at: None,
            },
            path: path.to_owned(),
        }
    }

    #[test]
    fn a_file_opens_its_folder_and_a_folder_opens_itself() {
        assert_eq!(
            destination(&hit("/docs/2024/a.txt", NodeKind::File, 1)),
            Folder::new("docs/2024")
        );
        assert_eq!(
            destination(&hit("/docs/2024", NodeKind::Directory, 0)),
            Folder::new("docs/2024")
        );
        assert_eq!(
            destination(&hit("/a.txt", NodeKind::File, 1)),
            Folder::root()
        );
    }

    #[test]
    fn a_hit_says_what_it_is_and_where() {
        assert_eq!(
            summary(&hit("/a.txt", NodeKind::File, 1_500)),
            "1.5 kB, in Files"
        );
        assert_eq!(
            summary(&hit("/docs/old", NodeKind::Directory, 0)),
            "Folder, in docs"
        );
    }
}
