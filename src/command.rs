use std::process::Command;

pub fn error_fn(e: &str) -> ! {
    eprintln!("Error: {e}");
    std::process::exit(0x1000);
}

#[macro_export]
macro_rules! error {
        ($($arg:tt)*) => {
        $crate::command::error_fn(std::format!("{}", std::format_args!($($arg)*)).as_str())
    };
}

#[must_use]
pub fn run(command_name: &str, args: &[&str]) -> String {
    match Command::new(command_name).args(args).output() {
        Ok(out) => match String::from_utf8(out.clone().stdout) {
            Ok(out_string) => out_string.trim().to_string(),
            Err(e) => error!("Failed to convert '{out:#?}' to string: {e}"),
        },
        Err(e) => error!("Failed to run '{command_name} {}' {e}", args.join(" ")),
    }
}
