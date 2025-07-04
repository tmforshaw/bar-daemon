use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Bluetooth {}

impl Bluetooth {
    fn get_state() -> Result<bool, Arc<ServerError>> {
        command::run("bluetooth", &[])?
            .split_whitespace()
            .nth(2)
            .map_or_else(|| todo!(), |value| Ok(value == "on"))
    }

    fn get_icon(state: bool) -> String {
        format!(
            "bluetooth-{}{}",
            if state { "active" } else { "disabled" },
            crate::ICON_EXT
        )
    }

    pub fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let state = Self::get_state()?;
        let icon = Self::get_icon(state);

        Ok(vec![
            ("state".to_string(), state.to_string()),
            ("icon".to_string(), icon),
        ])
    }

    pub fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        args.first().map_or_else(
            || Err(Arc::from(ServerError::EmptyArguments)),
            |argument| match argument.as_str() {
                "state" | "s" => Ok(vec_tup[0].1.clone()),
                "icon" | "i" => Ok(vec_tup[1].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["state", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
        )
    }
}
