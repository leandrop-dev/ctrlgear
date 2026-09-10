//! Button mapping functionality for SteelSeries mice

use crate::{Error, Result};
use std::str::FromStr;

/// Represents a physical button on the mouse
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Button1,
    Button2,
    Button3,
    Button4,
    Button5,
    Button6,
}

impl Button {
    /// Get the button ID used in the protocol
    pub fn id(&self) -> u8 {
        match self {
            Button::Button1 => 0x01,
            Button::Button2 => 0x02,
            Button::Button3 => 0x03,
            Button::Button4 => 0x04,
            Button::Button5 => 0x05,
            Button::Button6 => 0x06,
        }
    }

    /// Get all available buttons in order
    pub fn all() -> [Button; 6] {
        [
            Button::Button1,
            Button::Button2,
            Button::Button3,
            Button::Button4,
            Button::Button5,
            Button::Button6,
        ]
    }
}

impl FromStr for Button {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "button1" | "btn1" | "1" => Ok(Button::Button1),
            "button2" | "btn2" | "2" => Ok(Button::Button2),
            "button3" | "btn3" | "3" => Ok(Button::Button3),
            "button4" | "btn4" | "4" => Ok(Button::Button4),
            "button5" | "btn5" | "5" => Ok(Button::Button5),
            "button6" | "btn6" | "6" => Ok(Button::Button6),
            _ => Err(Error::InvalidParameter(format!("Invalid button: {}", s))),
        }
    }
}

/// Actions that can be assigned to a button
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtonAction {
    /// Button is disabled
    Disabled,
    /// Remapped to another mouse button
    MouseButton(Button),
    /// Switches between DPI presets
    DpiSwitch,
    /// Simulates scroll up
    ScrollUp,
    /// Simulates scroll down
    ScrollDown,
    /// Keep the default action for this button
    Default,
}

impl ButtonAction {
    /// Encode the action to a 5-byte field for the protocol
    pub fn encode(&self) -> [u8; 5] {
        match self {
            ButtonAction::Disabled => [0x00, 0x00, 0x00, 0x00, 0x00],
            ButtonAction::MouseButton(btn) => [btn.id(), 0x00, 0x00, 0x00, 0x00],
            ButtonAction::DpiSwitch => [0x30, 0x00, 0x00, 0x00, 0x00],
            ButtonAction::ScrollUp => [0x31, 0x00, 0x00, 0x00, 0x00],
            ButtonAction::ScrollDown => [0x32, 0x00, 0x00, 0x00, 0x00],
            ButtonAction::Default => {
                // This is a placeholder - the actual default will be filled in by the caller
                [0xFF, 0x00, 0x00, 0x00, 0x00]
            }
        }
    }
}

impl FromStr for ButtonAction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "disabled" | "disable" | "none" => Ok(ButtonAction::Disabled),
            "dpi" | "dpiswitch" | "dpi_switch" => Ok(ButtonAction::DpiSwitch),
            "scrollup" | "scroll_up" => Ok(ButtonAction::ScrollUp),
            "scrolldown" | "scroll_down" => Ok(ButtonAction::ScrollDown),
            "default" => Ok(ButtonAction::Default),
            _ => {
                // Try to parse as a button
                if let Ok(button) = Button::from_str(&lower) {
                    Ok(ButtonAction::MouseButton(button))
                } else {
                    Err(Error::InvalidParameter(format!(
                        "Invalid button action: {}. Valid options: disabled, button1-6, dpi, scrollup, scrolldown, default",
                        s
                    )))
                }
            }
        }
    }
}

/// Complete button mapping configuration
#[derive(Debug, Clone)]
pub struct ButtonMapping {
    mappings: [ButtonAction; 6],
}

impl ButtonMapping {
    /// Create a new button mapping with default values
    pub fn default_mapping() -> Self {
        ButtonMapping {
            mappings: [
                ButtonAction::MouseButton(Button::Button1),
                ButtonAction::MouseButton(Button::Button2),
                ButtonAction::MouseButton(Button::Button3),
                ButtonAction::MouseButton(Button::Button4),
                ButtonAction::MouseButton(Button::Button5),
                ButtonAction::DpiSwitch, // Button6 defaults to DPI switch
            ],
        }
    }

    /// Create a new button mapping from individual actions
    pub fn new(mappings: [ButtonAction; 6]) -> Self {
        ButtonMapping { mappings }
    }

    /// Set the action for a specific button
    pub fn set(&mut self, button: Button, action: ButtonAction) {
        let idx = button.id() as usize - 1;
        self.mappings[idx] = action;
    }

    /// Get the action for a specific button
    pub fn get(&self, button: Button) -> &ButtonAction {
        let idx = button.id() as usize - 1;
        &self.mappings[idx]
    }

    /// Encode the button mapping to a 30-byte packet for the device
    pub fn encode(&self) -> Vec<u8> {
        let defaults = Self::default_mapping();
        let mut packet = Vec::with_capacity(30);

        for (i, action) in self.mappings.iter().enumerate() {
            let bytes = match action {
                ButtonAction::Default => {
                    // Use the actual default for this button
                    defaults.mappings[i].encode()
                }
                _ => action.encode(),
            };
            packet.extend_from_slice(&bytes);
        }

        packet
    }

    /// Parse button mapping from a string format
    /// Format: "button1=button1,button2=button2,button3=scrollup,button6=dpi"
    pub fn from_string(s: &str) -> Result<Self> {
        let mut mapping = Self::default_mapping();

        if s.trim().is_empty() || s.to_lowercase() == "default" {
            return Ok(mapping);
        }

        for assignment in s.split(',') {
            let parts: Vec<&str> = assignment.split('=').map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(Error::InvalidParameter(format!(
                    "Invalid button assignment: '{}'. Expected format: 'button1=action'",
                    assignment
                )));
            }

            let button = Button::from_str(parts[0])?;
            let action = ButtonAction::from_str(parts[1])?;
            mapping.set(button, action);
        }

        Ok(mapping)
    }

    /// Get a human-readable description of the current mapping
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        for button in Button::all() {
            let action = self.get(button);
            let action_str = match action {
                ButtonAction::Disabled => "disabled".to_string(),
                ButtonAction::MouseButton(btn) => format!("{:?}", btn).to_lowercase(),
                ButtonAction::DpiSwitch => "dpi".to_string(),
                ButtonAction::ScrollUp => "scrollup".to_string(),
                ButtonAction::ScrollDown => "scrolldown".to_string(),
                ButtonAction::Default => "default".to_string(),
            };
            parts.push(format!("{:?}: {}", button, action_str));
        }
        parts.join(", ")
    }
}

impl Default for ButtonMapping {
    fn default() -> Self {
        Self::default_mapping()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_from_str() {
        assert_eq!(Button::from_str("button1").unwrap(), Button::Button1);
        assert_eq!(Button::from_str("btn3").unwrap(), Button::Button3);
        assert_eq!(Button::from_str("6").unwrap(), Button::Button6);
        assert!(Button::from_str("button7").is_err());
    }

    #[test]
    fn test_button_action_from_str() {
        assert_eq!(
            ButtonAction::from_str("disabled").unwrap(),
            ButtonAction::Disabled
        );
        assert_eq!(
            ButtonAction::from_str("dpi").unwrap(),
            ButtonAction::DpiSwitch
        );
        assert_eq!(
            ButtonAction::from_str("button2").unwrap(),
            ButtonAction::MouseButton(Button::Button2)
        );
    }

    #[test]
    fn test_button_action_encoding() {
        assert_eq!(ButtonAction::Disabled.encode(), [0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(
            ButtonAction::MouseButton(Button::Button1).encode(),
            [0x01, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            ButtonAction::DpiSwitch.encode(),
            [0x30, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            ButtonAction::ScrollUp.encode(),
            [0x31, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_default_mapping() {
        let mapping = ButtonMapping::default_mapping();
        assert_eq!(
            mapping.get(Button::Button1),
            &ButtonAction::MouseButton(Button::Button1)
        );
        assert_eq!(mapping.get(Button::Button6), &ButtonAction::DpiSwitch);
    }

    #[test]
    fn test_mapping_encode() {
        let mapping = ButtonMapping::default_mapping();
        let encoded = mapping.encode();
        assert_eq!(encoded.len(), 30); // 6 buttons × 5 bytes

        // Button 1 should be mapped to Button 1 (0x01)
        assert_eq!(encoded[0], 0x01);

        // Button 6 should be DPI switch (0x30)
        assert_eq!(encoded[25], 0x30);
    }

    #[test]
    fn test_mapping_from_string() {
        let mapping = ButtonMapping::from_string("button3=scrollup,button6=disabled").unwrap();
        assert_eq!(mapping.get(Button::Button3), &ButtonAction::ScrollUp);
        assert_eq!(mapping.get(Button::Button6), &ButtonAction::Disabled);
        // Others should still be default
        assert_eq!(
            mapping.get(Button::Button1),
            &ButtonAction::MouseButton(Button::Button1)
        );
    }

    #[test]
    fn test_mapping_from_string_default() {
        let mapping = ButtonMapping::from_string("default").unwrap();
        assert_eq!(mapping.get(Button::Button6), &ButtonAction::DpiSwitch);
    }

    #[test]
    fn test_mapping_swap_buttons() {
        let mapping = ButtonMapping::from_string("button1=button2,button2=button1").unwrap();
        assert_eq!(
            mapping.get(Button::Button1),
            &ButtonAction::MouseButton(Button::Button2)
        );
        assert_eq!(
            mapping.get(Button::Button2),
            &ButtonAction::MouseButton(Button::Button1)
        );
    }
}
