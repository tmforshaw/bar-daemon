use crate::command;
use crate::command::ServerError;

use std::sync::Arc;

pub struct Memory {}

impl Memory {
    fn get() -> Result<String, Arc<ServerError>> {
        Ok(command::run("free", &["-b"])?)
    }

    fn get_used_bytes(memory_command: &str) -> Result<f32, Arc<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(2) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Arc::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Arc::from(e),
                    })),
                },
                None => Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "used bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_available_bytes(memory_command: &str) -> Result<f32, Arc<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(1) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Arc::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Arc::from(e),
                    })),
                },
                None => Err(Arc::from(ServerError::NotInOutput {
                    looking_for: "available bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Arc::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_used_percent(memory_command: &str) -> Result<f32, Arc<ServerError>> {
        let used_bytes = Self::get_used_bytes(memory_command)?;
        let available_bytes = Self::get_available_bytes(memory_command)?;

        Ok((used_bytes / available_bytes) * 100f32)
    }

    fn get_icon() -> String {
        format!("ram{}", crate::ICON_EXT)
    }

    pub async fn update() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        Self::get_json_tuple()
    }

    pub fn get_json_tuple() -> Result<Vec<(String, String)>, Arc<ServerError>> {
        let memory_command = Self::get()?;
        let used_bytes = Self::get_used_bytes(&memory_command)?;
        let used_percent = Self::get_used_percent(&memory_command)?;
        let icon = Self::get_icon();

        Ok(vec![
            ("used_bytes".to_string(), used_bytes.to_string()),
            ("used_percent".to_string(), used_percent.to_string()),
            ("icon".to_string(), icon),
        ])
    }

    pub async fn parse_args(
        vec_tup: &[(String, String)],
        args: &[String],
    ) -> Result<String, Arc<ServerError>> {
        match args.get(0) {
            Some(argument) => match argument.as_str() {
                "used_bytes" | "used_b" | "ub" => Ok(vec_tup[0].1.clone()),
                "used_percent" | "used_p" | "up" => Ok(vec_tup[1].1.clone()),
                "icon" | "i" => Ok(vec_tup[2].1.clone()),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["used_bytes", "used_percent", "icon"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Arc::from(ServerError::EmptyArguments)),
        }
    }
}
