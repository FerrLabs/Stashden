use std::sync::Arc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use stashden_core::node::Node;

use super::parts::{self, Load};
use super::run;
use crate::desktop::Desktop;
use crate::failure::Failure;

const IMAGE_LIMIT: i64 = 16 * 1000 * 1000;
const TEXT_LIMIT: i64 = 256 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image(ImageFormat),
    Text,
    None,
}

const TEXT: [&str; 17] = [
    "css", "csv", "html", "js", "json", "log", "md", "markdown", "rs", "sh", "sql", "toml", "ts",
    "txt", "xml", "yaml", "yml",
];

pub fn kind(name: &str, size: i64) -> Kind {
    let extension = name
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map(|(_, extension)| extension.to_lowercase())
        .unwrap_or_default();
    let kind = match extension.as_str() {
        "png" | "apng" => Kind::Image(ImageFormat::Png),
        "jpg" | "jpeg" => Kind::Image(ImageFormat::Jpeg),
        "webp" => Kind::Image(ImageFormat::Webp),
        "gif" => Kind::Image(ImageFormat::Gif),
        "svg" => Kind::Image(ImageFormat::Svg),
        "bmp" => Kind::Image(ImageFormat::Bmp),
        text if TEXT.contains(&text) => Kind::Text,
        _ => Kind::None,
    };
    let limit = match kind {
        Kind::Image(_) => IMAGE_LIMIT,
        Kind::Text => TEXT_LIMIT,
        Kind::None => return Kind::None,
    };
    if size > limit { Kind::None } else { kind }
}

enum Shown {
    Image(Arc<Image>),
    Text(SharedString),
}

type Download = Arc<dyn Fn(&mut Window, &mut App)>;

pub struct Preview {
    content: Load<Shown>,
    downloaded: Download,
}

pub fn open(
    desktop: &Desktop,
    node: &Node,
    path: String,
    window: &mut Window,
    cx: &mut App,
    downloaded: impl Fn(&mut Window, &mut App) + 'static,
) {
    let kind = kind(&node.name, node.size);
    let preview = cx.new(|cx| {
        let content = if kind == Kind::None {
            Load::Failed("There is no preview for this kind of file.".to_owned())
        } else {
            let remote = desktop.remote();
            let work = async move {
                let bytes = remote?
                    .read(&path)
                    .await
                    .map_err(|error| Failure::from(error).about(&path))?;
                Ok::<_, Failure>(bytes.to_vec())
            };
            run(window, cx, work, move |this: &mut Preview, read, _, cx| {
                this.content = match read {
                    Ok(bytes) => Load::Ready(match kind {
                        Kind::Image(format) => {
                            Shown::Image(Arc::new(Image::from_bytes(format, bytes)))
                        }
                        Kind::Text | Kind::None => {
                            Shown::Text(String::from_utf8_lossy(&bytes).into_owned().into())
                        }
                    }),
                    Err(failure) => Load::Failed(format!("The preview did not load: {failure}")),
                };
                cx.notify();
            });
            Load::Loading
        };
        Preview {
            content,
            downloaded: Arc::new(downloaded),
        }
    });
    let title = node.name.clone();
    window.open_dialog(cx, move |shell, _, _| {
        shell
            .title(title.clone())
            .w(px(760.))
            .child(preview.clone())
    });
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.content {
            Load::Loading => parts::state("Loading the preview", cx).into_any_element(),
            Load::Failed(reason) => parts::state(reason.clone(), cx).into_any_element(),
            Load::Ready(Shown::Image(image)) => div()
                .flex()
                .justify_center()
                .child(img(Arc::clone(image)).max_h(px(520.)).max_w_full())
                .into_any_element(),
            Load::Ready(Shown::Text(text)) => div()
                .id("preview-text")
                .max_h(px(520.))
                .overflow_y_scroll()
                .p_2()
                .rounded_md()
                .bg(cx.theme().secondary)
                .font_family("monospace")
                .text_sm()
                .child(text.clone())
                .into_any_element(),
        };
        let downloaded = Arc::clone(&self.downloaded);
        div().flex().flex_col().gap_3().child(body).child(
            div()
                .flex()
                .flex_row()
                .justify_end()
                .gap_2()
                .child(
                    Button::new("preview-download")
                        .ghost()
                        .label("Download")
                        .on_click(move |_, window, cx| downloaded(window, cx)),
                )
                .child(
                    Button::new("preview-close")
                        .primary()
                        .label("Close")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::ImageFormat;

    use super::{IMAGE_LIMIT, Kind, TEXT_LIMIT, kind};

    #[test]
    fn images_and_text_are_previewed_by_extension_whatever_the_case() {
        assert_eq!(kind("photo.JPG", 10), Kind::Image(ImageFormat::Jpeg));
        assert_eq!(kind("notes.md", 10), Kind::Text);
        assert_eq!(kind("report.pdf", 10), Kind::None);
        assert_eq!(kind("archive.tar.gz", 10), Kind::None);
    }

    #[test]
    fn a_dotfile_or_an_oversized_file_gets_no_preview() {
        assert_eq!(kind(".txt", 10), Kind::None);
        assert_eq!(kind("huge.txt", TEXT_LIMIT + 1), Kind::None);
        assert_eq!(kind("edge.txt", TEXT_LIMIT), Kind::Text);
        assert_eq!(
            kind("big.png", TEXT_LIMIT + 1),
            Kind::Image(ImageFormat::Png)
        );
        assert_eq!(kind("huge.png", IMAGE_LIMIT + 1), Kind::None);
        assert_eq!(kind("edge.png", IMAGE_LIMIT), Kind::Image(ImageFormat::Png));
    }
}
