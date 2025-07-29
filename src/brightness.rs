use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
};

const MONITOR_ID: &str = "nvidia_wmi_ec_backlight";
const KEYBOARD_ID: &str = "asus::kbd_backlight";

#[derive(Subcommand)]
pub enum BrightnessGetCommands {
    #[command(alias = "mon", alias = "m")]
    Monitor,
    #[command(alias = "key", alias = "k")]
    Keyboard,
}

#[derive(Subcommand)]
pub enum BrightnessSetCommands {
    #[command(alias = "mon", alias = "m")]
    Monitor {
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    #[command(alias = "key", alias = "k")]
    Keyboard {
        #[arg()]
        value: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BrightnessItem {
    Monitor,
    Keyboard,
}

pub struct Brightness;

impl Brightness {
    fn get(device_id: &str) -> Result<f32, DaemonError> {
        let output = command::run("brightnessctl", &["-m", "-d", device_id, "i"])?;

        // Split the output by commas
        let output_split = output.split(",").map(ToString::to_string).collect::<Vec<_>>();

        // Get the current and maximum brightness values
        let current_brightness = output_split.get(2);
        let max_brightness = output_split.get(4);

        // Parse the values into integers, then get the floating point percentage
        Ok(
            if let (Some(current_brightness), Some(max_brightness)) = (current_brightness, max_brightness) {
                let current_value = current_brightness.parse::<u32>()? as f64;
                let max_value = max_brightness.parse::<u32>()? as f64;

                ((current_value / max_value) * 100.) as f32
            } else {
                return Err(DaemonError::ParseError(output));
            },
        )
    }

    pub fn get_monitor() -> Result<f32, DaemonError> {
        Self::get(MONITOR_ID)
    }

    pub fn get_keyboard() -> Result<f32, DaemonError> {
        Self::get(KEYBOARD_ID)
    }

    fn set(device_id: &str, percent_string: String) -> Result<(), DaemonError> {
        // Change the percentage based on the delta percentage
        let percent = if percent_string.starts_with("+") || percent_string.starts_with("-") {
            let delta_percent = percent_string.parse::<f64>()?;
            let current_percent = Self::get(device_id)? as f64;

            // Depending on the first char, add or subtract the percentage
            (current_percent
                + match percent_string.chars().next() {
                    Some('+') => delta_percent,
                    Some('-') => -delta_percent,
                    _ => 0.0,
                })
            .clamp(0.0, 100.0)
        } else {
            percent_string.parse::<f64>()?
        };

        // Set the percentage
        command::run("brightnessctl", &["-d", device_id, "s", format!("{percent}%").as_str()])?;

        Ok(())
    }

    pub fn set_monitor(percent: String) -> Result<(), DaemonError> {
        Self::set(MONITOR_ID, percent)
    }

    pub fn set_keyboard(percent: String) -> Result<(), DaemonError> {
        Self::set(KEYBOARD_ID, percent)
    }

    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let monitor_percent = Self::get_monitor()? as u32;

        Ok(vec![("brightness".to_string(), monitor_percent.to_string())])
    }

    pub fn match_get_commands(commands: BrightnessGetCommands) -> DaemonMessage {
        DaemonMessage::Get {
            item: match commands {
                BrightnessGetCommands::Monitor => DaemonItem::Brightness(BrightnessItem::Monitor),
                BrightnessGetCommands::Keyboard => DaemonItem::Brightness(BrightnessItem::Keyboard),
            },
        }
    }

    pub fn match_set_commands(commands: BrightnessSetCommands) -> DaemonMessage {
        match commands {
            BrightnessSetCommands::Monitor { value } => DaemonMessage::Set {
                item: DaemonItem::Brightness(BrightnessItem::Monitor),
                value,
            },
            BrightnessSetCommands::Keyboard { value } => DaemonMessage::Set {
                item: DaemonItem::Brightness(BrightnessItem::Keyboard),
                value: value.to_string(),
            },
        }
    }

    pub fn parse_item(
        item: DaemonItem,
        brightness_item: BrightnessItem,
        value: Option<String>,
    ) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            // Set value
            match brightness_item {
                BrightnessItem::Monitor => Self::set_monitor(value.clone())?,
                BrightnessItem::Keyboard => Self::set_keyboard(value.clone())?,
            };

            DaemonReply::Value { item, value }
        } else {
            // Get value
            match brightness_item {
                BrightnessItem::Monitor => DaemonReply::Value {
                    item,
                    value: Self::get_monitor()?.to_string(),
                },
                BrightnessItem::Keyboard => DaemonReply::Value {
                    item,
                    value: Self::get_keyboard()?.to_string(),
                },
            }
        })
    }
}
