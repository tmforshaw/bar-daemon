use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Bluetooth {}

impl Bluetooth {
    fn get_state() -> Result<bool, Arc<ServerError>> {
        match command::run("bluetooth", &[])?
            // .trim()
            .split_whitespace()
            .nth(2)
        {
            Some(value) => Ok(value == "on"),
            None => todo!(),
        }
    }

    fn get_icon(state: bool) -> String {
        format!(
            "{}/status/bluetooth-{}{}",
            crate::ICON_THEME_PATH,
            if state { "active" } else { "disabled" },
            crate::ICON_EXT
        )
    }

    pub async fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
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

    pub async fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "state" | "s" => Ok(vec_tup[0].1.clone()),
                "icon" | "i" => Ok(vec_tup[1].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["state", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Arc::from(ServerError::EmptyArguments)),
        }
    }
}
