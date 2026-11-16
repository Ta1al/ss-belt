pub mod generate;
pub mod x11;

use anyhow::Result;

use crate::error::user;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Shift,
    Control,
    Alt,
    Super,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accelerator {
    pub modifiers: Vec<Modifier>,
    pub key: String,
}

impl Accelerator {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(user("empty accelerator"));
        }
        let parts: Vec<&str> = trimmed.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return Err(user(format!("invalid accelerator `{input}`")));
        }
        let (key, modifier_parts) = parts.split_last().expect("non-empty");
        let mut modifiers = Vec::new();
        for part in modifier_parts {
            let modifier = match part.to_ascii_lowercase().as_str() {
                "shift" => Modifier::Shift,
                "ctrl" | "control" => Modifier::Control,
                "alt" | "mod1" | "option" => Modifier::Alt,
                "super" | "meta" | "win" | "mod4" | "cmd" => Modifier::Super,
                other => {
                    return Err(user(format!(
                        "unknown modifier `{other}` in `{input}` (use Shift, Ctrl, Alt or Mod4)"
                    )))
                }
            };
            if !modifiers.contains(&modifier) {
                modifiers.push(modifier);
            }
        }

        let key = key.to_string();
        if key.is_empty() {
            return Err(user(format!("`{input}` has no key")));
        }
        Ok(Self { modifiers, key })
    }

    pub fn label(&self) -> String {
        let mut parts: Vec<String> = self.modifiers.iter().map(|m| m.name().to_string()).collect();
        parts.push(self.key.clone());
        parts.join("+")
    }
}

impl Modifier {
    pub fn name(self) -> &'static str {
        match self {
            Modifier::Shift => "Shift",
            Modifier::Control => "Ctrl",
            Modifier::Alt => "Alt",
            Modifier::Super => "Mod4",
        }
    }
}

pub fn parse_accelerator(input: &str) -> Result<Accelerator> {
    Accelerator::parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_syntax() {
        let accel = Accelerator::parse("Mod4+Shift+4").unwrap();
        assert_eq!(accel.modifiers, vec![Modifier::Super, Modifier::Shift]);
        assert_eq!(accel.key, "4");
        assert_eq!(accel.label(), "Mod4+Shift+4");
    }

    #[test]
    fn modifiers_are_case_insensitive() {
        let accel = Accelerator::parse("ctrl+alt+t").unwrap();
        assert_eq!(accel.modifiers, vec![Modifier::Control, Modifier::Alt]);
        assert_eq!(accel.key, "t");
    }

    #[test]
    fn a_key_without_modifiers_is_allowed() {
        let accel = Accelerator::parse("Print").unwrap();
        assert!(accel.modifiers.is_empty());
        assert_eq!(accel.key, "Print");
    }

    #[test]
    fn unknown_modifiers_are_rejected() {
        let err = Accelerator::parse("Hyper+4").unwrap_err();
        assert!(err.to_string().contains("unknown modifier"));
    }

    #[test]
    fn empty_input_is_rejected() {
        assert!(Accelerator::parse("   ").is_err());
    }
}
