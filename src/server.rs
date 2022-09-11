use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::command::{call_and_retry, call_and_retry_async, ServerError};
use crate::memory::Memory;
use crate::volume::Volume;

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

enum ServerMessage {
    Update,
}

async fn socket_function(
    socket: Arc<Mutex<TcpStream>>,
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    result_mutex: Arc<Mutex<Result<(), Arc<ServerError>>>>,
    tx: mpsc::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        let mut buf = [0; 1024];

        let n = match socket.lock().await.read(&mut buf).await {
            Ok(n) if n == 0 => {
                *result_mutex.lock().await = Err(Arc::from(ServerError::SocketDisconnect));
                return;
            }
            Ok(n) => n,
            Err(e) => {
                *result_mutex.lock().await = Err(Arc::from(ServerError::SocketRead { e }));
                return;
            }
        };

        let message = match String::from_utf8(Vec::from(&buf[0..n])) {
            Ok(string) => string,
            Err(e) => {
                *result_mutex.lock().await = Err(Arc::from(ServerError::StringConversion {
                    debug_string: format!("{:?}", &buf[0..n]),
                    e,
                }));
                return;
            }
        };

        let args = message
            .split_ascii_whitespace()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>();

        let reply = match parse_args(&args, vol_mutex, bri_mutex, bat_mutex, mem_mutex, tx).await {
            Ok(reply) => reply,
            Err(e) => {
                *result_mutex.lock().await = Err(e);
                return;
            }
        };

        if let Some(r) = reply {
            if let Err(e) = socket.lock().await.write_all(r.as_bytes()).await {
                *result_mutex.lock().await = Err(Arc::from(ServerError::SocketWrite { e }));
            }
        };
    });
}

async fn socket_loop(
    listener: TcpListener,
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    result_mutex: Arc<Mutex<Result<(), Arc<ServerError>>>>,
    tx: mpsc::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        loop {
            let socket = match listener.accept().await {
                Ok((socket, _)) => Arc::from(Mutex::new(socket)),
                Err(e) => {
                    *result_mutex.clone().lock().await =
                        Err(Arc::from(ServerError::AddressInUse { e }));
                    return;
                }
            };

            let clone_vol_mutex_1 = vol_mutex.clone();
            let clone_bri_mutex_1 = bri_mutex.clone();
            let clone_bat_mutex_1 = bat_mutex.clone();
            let clone_mem_mutex_1 = mem_mutex.clone();

            let tx_clone_1 = tx.clone();
            let result_mutex_clone_1 = result_mutex.clone();

            socket_function(
                socket.clone(),
                clone_vol_mutex_1,
                clone_bri_mutex_1,
                clone_bat_mutex_1,
                clone_mem_mutex_1,
                result_mutex_clone_1,
                tx_clone_1,
            )
            .await;
        }
    });
}

async fn parse_args(
    args: &[String],
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    tx: mpsc::Sender<ServerMessage>,
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
                        "volume" | "vol" => Ok(Some(
                            Volume::parse_args(&vol_mutex.clone(), parseable_args).await?,
                        )),
                        "brightness" | "bri" => Ok(Some(
                            Brightness::parse_args(&bri_mutex.clone(), parseable_args).await?,
                        )),
                        "battery" | "bat" => Ok(Some(
                            Battery::parse_args(&bat_mutex.clone(), parseable_args).await?,
                        )),
                        "memory" | "mem" => Ok(Some(
                            Memory::parse_args(&mem_mutex.clone(), parseable_args).await?,
                        )),
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
                        "volume" | "vol" => Volume::update(&vol_mutex).await?,
                        "brightness" | "bri" => Brightness::update(&bri_mutex).await?,
                        "battery" | "bat" => Battery::update(&bat_mutex).await?,
                        "memory" | "mem" => Memory::update(&mem_mutex).await?,
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

                let full_json = get_all_json(vol_mutex, bri_mutex, bat_mutex, mem_mutex).await?;

                if tx.send(ServerMessage::Update).await.is_err() {
                    return Err(Arc::from(ServerError::ChannelSend { message: full_json }));
                };

                Ok(None)
            }
            incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: vec!["get", "listen", "update"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        },
        None => Err(Arc::from(ServerError::EmptyArguments)),
    }
}

pub async fn start() -> Result<(), Arc<ServerError>> {
    let listener = match call_and_retry_async(|| async {
        TcpListener::bind(crate::IP_AND_PORT).await
    })
    .await
    {
        Some(Ok(handle)) => handle,
        Some(Err(e)) => return Err(Arc::from(ServerError::AddressInUse { e })),
        None => return Err(Arc::from(ServerError::RetryError)),
    };

    // Make a channel to listen for updates and send the json
    let (tx, mut rx) = mpsc::channel(32);
    let tx_clone_1 = tx.clone();

    let result_mutex = Arc::from(Mutex::new(Ok::<(), Arc<ServerError>>(())));
    let result_mutex_clone_0 = result_mutex.clone();

    tokio::spawn(async move {
        use std::time::Duration;

        loop {
            // Check for errors
            if let Err(e) = &*result_mutex_clone_0.lock().await {
                eprintln!("{e}");
            }

            if tx_clone_1.send(ServerMessage::Update).await.is_err() {
                eprintln!("Failed to send update message");
            }

            std::thread::sleep(Duration::from_millis(1500));
        }
    });

    let vol_mutex = Arc::new(Mutex::new(match call_and_retry(Volume::get_json_tuple) {
        Some(Ok(vol_out)) => vol_out,
        Some(Err(e)) => return Err(e),
        None => return Err(Arc::from(ServerError::RetryError)),
    }));

    let bri_mutex = Arc::new(Mutex::new(Brightness::get_json_tuple()?));
    let bat_mutex = Arc::new(Mutex::new(Battery::get_json_tuple()?));
    let mem_mutex = Arc::new(Mutex::new(Memory::get_json_tuple()?));

    let clone_vol_mutex_1 = vol_mutex.clone();
    let clone_bri_mutex_1 = bri_mutex.clone();
    let clone_bat_mutex_1 = bat_mutex.clone();
    let clone_mem_mutex_1 = mem_mutex.clone();

    socket_loop(
        listener,
        vol_mutex.clone(),
        bri_mutex.clone(),
        bat_mutex.clone(),
        mem_mutex.clone(),
        result_mutex.clone(),
        tx.clone(),
    )
    .await;

    while let Some(val) = rx.recv().await {
        match val {
            ServerMessage::Update => {
                if let Err(e) = Battery::update(&clone_bat_mutex_1).await {
                    eprintln!("{e}");
                }

                if let Err(e) = Memory::update(&clone_mem_mutex_1).await {
                    eprintln!("{e}");
                }

                match get_all_json(
                    clone_vol_mutex_1.clone(),
                    clone_bri_mutex_1.clone(),
                    clone_bat_mutex_1.clone(),
                    clone_mem_mutex_1.clone(),
                )
                .await
                {
                    Ok(json) => println!("{json}"),
                    Err(e) => eprintln!("{e}"),
                };
            }
        }
    }

    Ok(())
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
