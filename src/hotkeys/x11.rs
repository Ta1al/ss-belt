use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};
use x11rb::protocol::Event;

use crate::config::Config;
use crate::error::user_hint;
use crate::hotkeys::{Accelerator, Modifier};

/// Resolve an X11 keysym for the key names ss-belt accepts.
fn keysym(key: &str) -> Option<u32> {
    let lower = key.to_ascii_lowercase();
    let named = match lower.as_str() {
        "return" | "enter" => Some(0xff0d),
        "print" | "printscreen" | "sysrq" => Some(0xff61),
        "space" => Some(0x0020),
        "escape" | "esc" => Some(0xff1b),
        "tab" => Some(0xff09),
        "backspace" => Some(0xff08),
        "delete" | "del" => Some(0xffff),
        "insert" | "ins" => Some(0xff63),
        "home" => Some(0xff50),
        "end" => Some(0xff57),
        "pageup" | "prior" => Some(0xff55),
        "pagedown" | "next" => Some(0xff56),
        "left" => Some(0xff51),
        "up" => Some(0xff52),
        "right" => Some(0xff53),
        "down" => Some(0xff54),
        "menu" => Some(0xff67),
        "pause" => Some(0xff13),
        "comma" => Some(0x002c),
        "period" => Some(0x002e),
        "slash" => Some(0x002f),
        "semicolon" => Some(0x003b),
        "minus" => Some(0x002d),
        "equal" => Some(0x003d),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    if let Some(rest) = lower.strip_prefix('f') {
        if let Ok(number) = rest.parse::<u32>() {
            if (1..=24).contains(&number) {
                return Some(0xffbe + number - 1);
            }
        }
    }
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii() => Some(c.to_ascii_lowercase() as u32),
        _ => None,
    }
}

fn modifier_mask(modifiers: &[Modifier]) -> u16 {
    let mut mask = 0u16;
    for modifier in modifiers {
        mask |= match modifier {
            Modifier::Shift => 1,
            Modifier::Control => 4,
            Modifier::Alt => 8,
            Modifier::Super => 64,
        };
    }
    mask
}

struct Binding {
    action: String,
    keycode: u8,
    mask: u16,
}

/// Grab every configured hotkey on the X11 root window and call `handler` when
/// one fires. Blocks forever; run this on a dedicated thread.
pub fn grab<F>(config: &Config, mut handler: F) -> Result<()>
where
    F: FnMut(&str),
{
    let (connection, screen_num) = x11rb::connect(None).map_err(|err| {
        user_hint(
            format!("cannot connect to the X server: {err}"),
            "X11 hotkeys need DISPLAY. On Wayland, use `ss-belt hotkeys --print <desktop>` \
             and let the compositor own the keys",
        )
    })?;

    let setup = connection.setup();
    let root = setup.roots[screen_num].root;
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;
    let count = max_keycode - min_keycode + 1;

    let mapping = connection
        .get_keyboard_mapping(min_keycode, count)?
        .reply()
        .context("reading the keyboard mapping")?;

    let per_keycode = mapping.keysyms_per_keycode as usize;
    let mut keycode_for: HashMap<u32, u8> = HashMap::new();
    for (index, chunk) in mapping.keysyms.chunks(per_keycode).enumerate() {
        for symbol in chunk {
            if *symbol != 0 {
                keycode_for.entry(*symbol).or_insert(min_keycode + index as u8);
            }
        }
    }

    let mut bindings: Vec<Binding> = Vec::new();
    let mut requested = 0usize;
    for action in crate::config::HOTKEY_ACTIONS {
        let Some(text) = config.hotkeys.get(action) else { continue };
        let accelerator = Accelerator::parse(text)?;
        let Some(symbol) = keysym(&accelerator.key) else {
            tracing::warn!("hotkey {action}: key `{}` is not supported on X11", accelerator.key);
            continue;
        };
        let Some(keycode) = keycode_for.get(&symbol).copied() else {
            tracing::warn!("hotkey {action}: `{}` is not present on this keyboard", accelerator.key);
            continue;
        };
        requested += 1;
        let base = modifier_mask(&accelerator.modifiers);
        // Lock and NumLock change the reported state; grab the combinations.
        for extra in [0u16, 2, 16, 18] {
            let mask = base | extra;
            connection.grab_key(true, root, ModMask::from(mask), keycode, GrabMode::ASYNC, GrabMode::ASYNC)?;
            bindings.push(Binding { action: action.to_string(), keycode, mask });
        }
    }
    connection.flush()?;

    if bindings.is_empty() {
        return Err(user_hint(
            "none of the configured hotkeys could be grabbed",
            "check `ss-belt hotkeys --check` and set keys your keyboard actually has",
        ));
    }

    tracing::info!("grabbed {} hotkey(s) on X11", requested);

    let mut last_fire: Option<(u8, Instant)> = None;
    loop {
        let event = connection.wait_for_event().context("waiting for X11 events")?;
        if let Event::KeyPress(press) = event {
            let state = press.state;
            let Some(binding) = bindings
                .iter()
                .find(|binding| binding.keycode == press.detail && binding.mask == u16::from(state))
            else {
                continue;
            };
            // Ignore auto-repeat: X sends repeated KeyPress events.
            if let Some((keycode, at)) = last_fire {
                if keycode == press.detail && at.elapsed() < Duration::from_millis(300) {
                    continue;
                }
            }
            last_fire = Some((press.detail, Instant::now()));
            handler(&binding.action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keysyms_cover_letters_digits_and_function_keys() {
        assert_eq!(keysym("4"), Some(0x34));
        assert_eq!(keysym("q"), Some(0x71));
        assert_eq!(keysym("Q"), Some(0x71));
        assert_eq!(keysym("F5"), Some(0xffbe + 4));
        assert_eq!(keysym("Print"), Some(0xff61));
        assert_eq!(keysym("nope!"), None);
    }

    #[test]
    fn masks_match_x11_modifier_bits() {
        assert_eq!(modifier_mask(&[Modifier::Shift]), 1);
        assert_eq!(modifier_mask(&[Modifier::Control]), 4);
        assert_eq!(modifier_mask(&[Modifier::Alt]), 8);
        assert_eq!(modifier_mask(&[Modifier::Super, Modifier::Shift]), 65);
    }
}
