mod battery;
mod brightness;
pub mod command;
mod memory;
mod sender;
mod server;
mod volume;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().collect::<Vec<String>>();

    // No arguments entered
    if args.len() <= 1 {
        // Attempt to open the server
        server::start().await
    } else {
        match args[1].as_str() {
            "send" => {
                // Allow sending of messages
                sender::start(&args.split_off(2)).await
            }
            "daemon" => {
                // Attempt to open the server
                server::start().await
            }
            _ => error!("Please enter 'send' or 'daemon'"),
        }
    }
}
