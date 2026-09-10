//! CtrlGear - A Rust library for managing SteelSeries hardware
//!
//! This library provides an interface for configuring SteelSeries gaming peripherals,
//! with a focus on mice like the Rival 3 Wireless.

pub mod buttons;
pub mod device;
pub mod error;
pub mod hid;
pub mod protocol;
pub mod registry;

pub use buttons::{Button, ButtonAction, ButtonMapping};
pub use error::{Error, Result};
