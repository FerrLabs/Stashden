use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const OPEN: &str = "open";
const QUIT: &str = "quit";
const ICON: &[u8] = include_bytes!("../icons/32x32.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    Open,
    Quit,
}

pub struct Tray {
    pub icon: TrayIcon,
    pub asked: UnboundedReceiver<Asked>,
}

pub fn install() -> Result<Tray, String> {
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id(OPEN, "Open Stashden", true, None),
        &MenuItem::with_id(QUIT, "Quit", true, None),
    ])
    .map_err(|error| error.to_string())?;

    let icon = TrayIconBuilder::new()
        .with_tooltip("Stashden")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_icon(icon()?)
        .build()
        .map_err(|error| error.to_string())?;

    let (sender, asked) = unbounded();
    let from_menu = sender.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(asked) = menu_choice(event.id.as_ref()) {
            let _ = from_menu.unbounded_send(asked);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = sender.unbounded_send(Asked::Open);
        }
    }));

    Ok(Tray { icon, asked })
}

fn menu_choice(id: &str) -> Option<Asked> {
    match id {
        OPEN => Some(Asked::Open),
        QUIT => Some(Asked::Quit),
        _ => None,
    }
}

fn icon() -> Result<Icon, String> {
    let image = image::load_from_memory(ICON)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_two_menu_entries_ask_for_anything() {
        assert_eq!(menu_choice(OPEN), Some(Asked::Open));
        assert_eq!(menu_choice(QUIT), Some(Asked::Quit));
        assert_eq!(menu_choice("separator"), None);
    }

    #[test]
    fn the_bundled_icon_decodes() {
        assert!(icon().is_ok());
    }
}
