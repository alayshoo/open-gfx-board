use anyhow::Result;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn fire_shortcut(shortcut: &str) -> Result<()> {
    let key = match shortcut {
        "F13" => Key::Other(0x7C),
        "F14" => Key::Other(0x7D),
        "F15" => Key::Other(0x7E),
        "F16" => Key::Other(0x7F),
        "F17" => Key::Other(0x80),
        "F18" => Key::Other(0x81),
        "F19" => Key::Other(0x82),
        "F20" => Key::Other(0x83),
        "F21" => Key::Other(0x84),
        "F22" => Key::Other(0x85),
        "F23" => Key::Other(0x86),
        "F24" => Key::Other(0x87),
        _ => return Err(anyhow::anyhow!("Unknown shortcut: {shortcut}")),
    };
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(key, Direction::Click)?;
    Ok(())
}
