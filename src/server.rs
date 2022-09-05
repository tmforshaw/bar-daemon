use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::command::ServerError;
use crate::memory::Memory;
use crate::volume::Volume;

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

async fn socket_function(
    socket: &mut tokio::net::TcpStream,
    buf: &mut [u8],
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    ram_mutex: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<(), Arc<ServerError>> {
    let n = match socket.read(buf).await {
        Ok(n) if n == 0 => return Err(Arc::from(ServerError::SocketDisconnect)),
        Ok(n) => n,
        Err(e) => return Err(Arc::from(ServerError::SocketRead { e })),
    };

    let message = match String::from_utf8(Vec::from(&buf[0..n])) {
        Ok(string) => string,
        Err(e) => {
            return Err(Arc::from(ServerError::StringConversion {
                debug_string: format!("{:?}", &buf[0..n]),
                e,
            }))
        }
    };

    let mut args = message
        .split_ascii_whitespace()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();

    let parseable_args = args.split_off(2);

    let reply = match args.get(0) {
        Some(command) => match command.as_str() {
            "get" => match args.get(1) {
                Some(argument) => match argument.as_str() {
                    "volume" | "vol" => {
                        let lock = vol_mutex.lock().await;

                        Some(Volume::parse_args(&lock.clone(), &parseable_args)?)
                    }
                    "brightness" | "bri" => {
                        let lock = bri_mutex.lock().await;

                        Some(Brightness::parse_args(&lock.clone(), &parseable_args)?)
                    }
                    "battery" | "bat" => {
                        let lock = bat_mutex.lock().await;

                        Some(Battery::parse_args(&lock.clone(), &parseable_args)?)
                    }
                    "memory" | "mem" => Some(Memory::parse_args(&parseable_args)?),
                    incorrect => {
                        return Err(Arc::from(ServerError::IncorrectArgument {
                            incorrect: incorrect.to_string(),
                            valid: vec!["volume", "brightness", "battery", "memory"]
                                .iter()
                                .map(std::string::ToString::to_string)
                                .collect(),
                        }))
                    }
                },
                None => return Err(Arc::from(ServerError::EmptyArguments)),
            },
            "update" => match args.get(1) {
                Some(argument) => match argument.as_str() {
                    "volume" | "vol" => {
                        let vol = Volume::get_json_tuple()?;

                        let mut lock = vol_mutex.lock().await;
                        *lock = vol.clone();

                        println!("{:?}", *lock);

                        None
                    }
                    "brightness" | "bri" => {
                        let bri = Brightness::get_json_tuple()?;

                        let mut lock = bri_mutex.lock().await;
                        *lock = bri.clone();

                        None
                    }
                    incorrect => {
                        return Err(Arc::from(ServerError::IncorrectArgument {
                            incorrect: incorrect.to_string(),
                            valid: vec!["volume", "brightness"]
                                .iter()
                                .map(std::string::ToString::to_string)
                                .collect(),
                        }))
                    }
                },
                None => match get_all_json() {
                    Ok(json) => {
                        println!("{json}");
                        None
                    }
                    Err(e) => return Err(Arc::from(e)),
                },
            },
            incorrect => {
                return Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["get", "update"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                }))
            }
        },
        None => return Err(Arc::from(ServerError::EmptyArguments)),
    };

    if let Some(r) = reply {
        if let Err(e) = socket.write_all(r.as_bytes()).await {
            return Err(Arc::from(ServerError::SocketWrite { e }));
        }
    };

    Ok(())
}

pub async fn start() -> Result<(), Arc<ServerError>> {
    let listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(handle) => handle,
        Err(e) => return Err(Arc::from(ServerError::AddressInUse { e })),
    };

    // tokio::spawn(async move {
    //     use std::time::Duration;

    //     loop {
    //         match get_all_json() {
    //             Ok(json) => {
    //                 println!("{json}");
    //             }
    //             Err(e) => eprintln!("{e}"),
    //         };

    //         std::thread::sleep(Duration::from_millis(1500));
    //     }
    // });

    let vol_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Volume::get_json_tuple().unwrap()));

    let bri_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Brightness::get_json_tuple().unwrap()));

    let bat_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Battery::get_json_tuple().unwrap()));

    let ram_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Brightness::get_json_tuple().unwrap()));

    loop {
        let mut socket = match listener.accept().await {
            Ok((socket, _)) => socket,
            Err(e) => return Err(Arc::from(ServerError::AddressInUse { e })),
        };

        let clone_vol_mutex = vol_mutex.clone();
        let clone_bri_mutex = bri_mutex.clone();
        let clone_bat_mutex = bat_mutex.clone();
        let clone_ram_mutex = ram_mutex.clone();

        if let Err(join_error) = tokio::spawn(async move {
            let mut buf = [0; 1024];

            if let Err(e) = socket_function(
                &mut socket,
                &mut buf,
                clone_vol_mutex,
                clone_bri_mutex,
                clone_bat_mutex,
                clone_ram_mutex,
            )
            .await
            {
                eprintln!("{e}");
                // if let Err(write_e) = futures::join!(socket.write_all(format!("{e}").as_bytes())).0
                // {
                //     eprintln!("Could not write errors to socket: {write_e}");
                // }
            }
        })
        .await
        {
            return Err(Arc::from(ServerError::SocketJoin { e: join_error }));
        }
    }
}

fn get_all_json() -> Result<String, Box<ServerError>> {
    let volume = match Volume::get_json() {
        Ok(vol) => vol,
        Err(e) => return Err(e),
    };

    let brightness = match Brightness::get_json() {
        Ok(bri) => bri,
        Err(e) => return Err(e),
    };

    let battery = match Battery::get_json() {
        Ok(bat) => bat,
        Err(e) => return Err(e),
    };

    let memory = match Memory::get_json() {
        Ok(mem) => mem,
        Err(e) => return Err(e),
    };

    Ok(format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}}}",
        volume, brightness, battery, memory
    ))
}
