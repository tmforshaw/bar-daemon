use crate::command;
use crate::command::ServerError;

pub struct Memory {}

impl Memory {
    fn get() -> Result<String, Box<ServerError>> {
        match command::run("free", &["-b"]) {
            Ok(output) => Ok(output),
            Err(e) => Err(Box::from(e)),
        }
    }

    fn get_used_bytes(memory_command: &str) -> Result<f32, Box<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(2) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Box::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Box::from(e),
                    })),
                },
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "used bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_available_bytes(memory_command: &str) -> Result<f32, Box<ServerError>> {
        match memory_command.split_terminator('\n').nth(1) {
            Some(line) => match line.split_ascii_whitespace().nth(1) {
                Some(string) => match string.trim().parse::<f32>() {
                    Ok(float_val) => Ok(float_val),
                    Err(e) => Err(Box::from(ServerError::StringParse {
                        debug_string: string.to_string(),
                        ty: "float".to_string(),
                        e: Box::from(e),
                    })),
                },
                None => Err(Box::from(ServerError::NotInOutput {
                    looking_for: "available bytes".to_string(),
                    output: line.to_string(),
                })),
            },
            None => Err(Box::from(ServerError::NotInOutput {
                looking_for: "memory".to_string(),
                output: memory_command.to_string(),
            })),
        }
    }

    fn get_used_percent(memory_command: &str) -> Result<f32, Box<ServerError>> {
        let used_bytes = match Self::get_used_bytes(memory_command) {
            Ok(ub) => ub,
            Err(e) => return Err(e),
        };

        let available_bytes = match Self::get_available_bytes(memory_command) {
            Ok(ab) => ab,
            Err(e) => return Err(e),
        };

        Ok((used_bytes / available_bytes) * 100f32)
    }

    pub fn get_json() -> Result<String, Box<ServerError>> {
        let memory_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let used_bytes = match Self::get_used_bytes(&memory_command) {
            Ok(ub) => ub,
            Err(e) => return Err(e),
        };

        let used_percent = match Self::get_used_percent(&memory_command) {
            Ok(up) => up,
            Err(e) => return Err(e),
        };

        Ok(format!(
            "{{\"used_bytes\": {}, \"used_percent\": \"{}\"}}",
            used_bytes, used_percent,
        ))
    }

    pub fn parse_args(args: &[String]) -> Result<String, Box<ServerError>> {
        let memory_command = match Self::get() {
            Ok(output) => output,
            Err(e) => return Err(e),
        };

        let used_bytes = match Self::get_used_bytes(&memory_command) {
            Ok(ub) => ub,
            Err(e) => return Err(e),
        };

        let used_percent = match Self::get_used_percent(&memory_command) {
            Ok(up) => up,
            Err(e) => return Err(e),
        };

        match args[0].as_str() {
            "used_bytes" | "used_b" | "ub" => Ok(used_bytes.to_string()),
            "used_percent" | "used_p" | "up" => Ok(used_percent.to_string()),
            incorrect => Err(Box::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: ["used_bytes", "used_percent"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        }
    }
}
