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
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
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

    let args = message
        .split_ascii_whitespace()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();

    let reply = parse_args(&args, vol_mutex, bri_mutex, bat_mutex, mem_mutex).await?;

    if let Some(r) = reply {
        if let Err(e) = socket.write_all(r.as_bytes()).await {
            return Err(Arc::from(ServerError::SocketWrite { e }));
        }
    };

    Ok(())
}

async fn parse_args(
    args: &[String],
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<Option<String>, Arc<ServerError>> {
    match args.get(0) {
        Some(command) => match command.as_str() {
            "get" => {
                let parseable_args = if args.len() > 2 {
                    args.split_at(2).1
                } else {
                    return Err(Arc::from(ServerError::EmptyArguments));
                };

                match args.get(1) {
                    Some(argument) => match argument.as_str() {
                        "volume" | "vol" => {
                            let lock = vol_mutex.lock().await;

                            Ok(Some(Volume::parse_args(&lock.clone(), parseable_args)?))
                        }
                        "brightness" | "bri" => {
                            let lock = bri_mutex.lock().await;

                            Ok(Some(Brightness::parse_args(&lock.clone(), parseable_args)?))
                        }
                        "battery" | "bat" => {
                            let lock = bat_mutex.lock().await;

                            Ok(Some(Battery::parse_args(&lock.clone(), parseable_args)?))
                        }
                        "memory" | "mem" => {
                            let lock = mem_mutex.lock().await;
                            Ok(Some(Memory::parse_args(&lock.clone(), parseable_args)?))
                        }
                        incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                            incorrect: incorrect.to_string(),
                            valid: vec!["volume", "brightness", "battery", "memory"]
                                .iter()
                                .map(std::string::ToString::to_string)
                                .collect(),
                        })),
                    },
                    None => Err(Arc::from(ServerError::EmptyArguments)),
                }
            }
            "update" => {
                match args.get(1) {
                    Some(argument) => match argument.as_str() {
                        "volume" | "vol" => {
                            let vol = Volume::get_json_tuple()?;

                            let mut lock = vol_mutex.lock().await;
                            *lock = vol.clone();
                        }
                        "brightness" | "bri" => {
                            let bri = Brightness::get_json_tuple()?;

                            let mut lock = bri_mutex.lock().await;
                            *lock = bri.clone();
                        }
                        "battery" | "bat" => {
                            let bat = Battery::get_json_tuple()?;

                            let mut lock = bat_mutex.lock().await;
                            *lock = bat.clone();
                        }
                        "memory" | "mem" => {
                            let mem = Memory::get_json_tuple()?;

                            let mut lock = mem_mutex.lock().await;
                            *lock = mem.clone();
                        }
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
                    None => {}
                };

                println!(
                    "{}",
                    get_all_json(vol_mutex, bri_mutex, bat_mutex, mem_mutex).await?
                );

                Ok(None)
            }
            incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["get", "update"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        },
        None => Err(Arc::from(ServerError::EmptyArguments)),
    }
}

pub async fn start() -> Result<(), Arc<ServerError>> {
    let listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(handle) => handle,
        Err(e) => return Err(Arc::from(ServerError::AddressInUse { e })),
    };

    let vol_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Volume::get_json_tuple()?));

    let bri_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Brightness::get_json_tuple()?));

    let bat_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Battery::get_json_tuple()?));

    let mem_mutex: Arc<Mutex<Vec<(String, String)>>> =
        Arc::new(Mutex::new(Memory::get_json_tuple()?));

    let clone_vol_mutex_1 = vol_mutex.clone();
    let clone_bri_mutex_1 = bri_mutex.clone();
    let clone_bat_mutex_1 = bat_mutex.clone();
    let clone_mem_mutex_1 = mem_mutex.clone();

    tokio::spawn(async move {
        use std::time::Duration;

        loop {
            match get_all_json(
                clone_vol_mutex_1.clone(),
                clone_bri_mutex_1.clone(),
                clone_bat_mutex_1.clone(),
                clone_mem_mutex_1.clone(),
            )
            .await
            {
                Ok(json) => {
                    println!("{json}");
                }
                Err(e) => eprintln!("{e}"),
            };

            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    loop {
        let mut socket = match listener.accept().await {
            Ok((socket, _)) => socket,
            Err(e) => return Err(Arc::from(ServerError::AddressInUse { e })),
        };

        let clone_vol_mutex_2 = vol_mutex.clone();
        let clone_bri_mutex_2 = bri_mutex.clone();
        let clone_bat_mutex_2 = bat_mutex.clone();
        let clone_mem_mutex_2 = mem_mutex.clone();

        if let Err(join_error) = tokio::spawn(async move {
            let mut buf = [0; 1024];

            if let Err(e) = socket_function(
                &mut socket,
                &mut buf,
                clone_vol_mutex_2,
                clone_bri_mutex_2,
                clone_bat_mutex_2,
                clone_mem_mutex_2,
            )
            .await
            {
                if let Err(write_e) = socket.write_all(format!("{e}").as_bytes()).await {
                    return Err(Arc::from(ServerError::SocketWrite { e: write_e }));
                }
            };

            Ok(())
        })
        .await
        {
            return Err(Arc::from(ServerError::SocketJoin { e: join_error }));
        }
    }
}

fn get_json_from_tuple(vec_tup: &[(String, String)]) -> String {
    let joined_string = vec_tup
        .iter()
        .map(|t| format!("\"{}\": \"{}\"", t.0, t.1))
        .collect::<Vec<String>>()
        .join(", ");

    format!("{{{}}}", joined_string)
}

async fn get_all_json(
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
) -> Result<String, Box<ServerError>> {
    let volume_tup = vol_mutex.lock().await.clone();
    let brightness_tup = bri_mutex.lock().await.clone();
    let battery_tup = bat_mutex.lock().await.clone();
    let memory_tup = mem_mutex.lock().await.clone();

    Ok(format!(
        "{{\"volume\": {}, \"brightness\": {}, \"battery\": {}, \"memory\": {}}}",
        get_json_from_tuple(&volume_tup),
        get_json_from_tuple(&brightness_tup),
        get_json_from_tuple(&battery_tup),
        get_json_from_tuple(&memory_tup),
    ))
}
