use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::command::{
    call_and_retry, call_and_retry_async, get_all_json, mpsc_send, socket_read, socket_write,
    ServerError,
};
use crate::memory::Memory;
use crate::volume::Volume;

use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch, Mutex};

#[derive(Clone)]
enum ServerMessage {
    UpdateAll,
    GetVol { args: Vec<String> },
    UpdateVol,
    GetBri { args: Vec<String> },
    UpdateBri,
    GetBat { args: Vec<String> },
    UpdateBat,
    GetMem { args: Vec<String> },
    UpdateMem,
}

impl std::fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UpdateAll => "Update server".to_string(),
                Self::GetVol { args } => format!("Get volume: {args:?}"),
                Self::UpdateVol => "Update volume".to_string(),
                Self::GetBri { args } => format!("Get brightness: {args:?}"),
                Self::UpdateBri => "Update brightness".to_string(),
                Self::GetBat { args } => format!("Get battery: {args:?}"),
                Self::UpdateBat => "Update battery".to_string(),
                Self::GetMem { args } => format!("Get memory: {args:?}"),
                Self::UpdateMem => "Update memory".to_string(),
            }
        )
    }
}

async fn process_socket_message(
    socket: Arc<Mutex<TcpStream>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
) -> Option<Vec<String>> {
    let message = match socket_read(socket).await {
        Ok(m) => m,
        Err(e) => {
            if let Err(e) = mpsc_send(error_tx, e).await {
                eprintln!("Could not send error via channel: {e}");
            }

            return None;
        }
    };

    Some(
        message
            .split_ascii_whitespace()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>(),
    )
}

async fn socket_loop(
    listener: TcpListener,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerMessage>,
    server_response_rx: Arc<Mutex<watch::Receiver<Result<String, Arc<ServerError>>>>>,
    listener_rx: Arc<Mutex<watch::Receiver<String>>>,
) {
    loop {
        let socket = match listener.accept().await {
            Ok((socket, _)) => Arc::from(Mutex::new(socket)),
            Err(e) => {
                if let Err(e) = error_tx
                    .clone()
                    .send(Arc::from(ServerError::AddressInUse { e }))
                    .await
                {
                    eprintln!("Could not send error via channel: {e}");
                };
                return;
            }
        };

        let server_tx_1 = server_tx.clone();
        let server_response_rx_1 = server_response_rx.clone();
        let listener_rx_1 = listener_rx.clone();
        let error_tx_1 = error_tx.clone();

        tokio::spawn(async move {
            let args = match process_socket_message(socket.clone(), error_tx_1.clone()).await {
                Some(args) => args,
                None => return,
            };

            let reply = match parse_args(
                socket.clone(),
                &args,
                error_tx_1.clone(),
                server_tx_1,
                server_response_rx_1,
                listener_rx_1,
            )
            .await
            {
                Ok(reply) => reply,
                Err(e) => {
                    if let Err(e) = socket_write(socket, e.to_string().as_bytes()).await {
                        if let Err(e) = mpsc_send(error_tx_1, e).await {
                            eprintln!("Could not send error via channel: {e}");
                        }
                    }
                    return;
                }
            };

            if let Some(r) = reply {
                if let Err(e) = socket_write(socket, r.as_bytes()).await {
                    if let Err(e) = mpsc_send(error_tx_1, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            };
        });
    }
}

async fn parse_args(
    socket: Arc<Mutex<TcpStream>>,
    args: &[String],
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerMessage>,
    server_response_rx: Arc<Mutex<watch::Receiver<Result<String, Arc<ServerError>>>>>,
    listener_rx: Arc<Mutex<watch::Receiver<String>>>,
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
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ServerMessage::GetVol {
                                    args: parseable_args.to_vec(),
                                },
                            )
                            .await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }

                            if server_response_rx.lock().await.changed().await.is_ok() {
                                let value = server_response_rx.lock().await.borrow().clone()?;

                                Ok(Some(value))
                            } else {
                                Err(Arc::from(ServerError::ChannelSend {
                                    message: ServerMessage::GetVol {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "brightness" | "bri" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ServerMessage::GetBri {
                                    args: parseable_args.to_vec(),
                                },
                            )
                            .await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }

                            if server_response_rx.lock().await.changed().await.is_ok() {
                                let value = server_response_rx.lock().await.borrow().clone()?;

                                Ok(Some(value))
                            } else {
                                Err(Arc::from(ServerError::ChannelSend {
                                    message: ServerMessage::GetBri {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "battery" | "bat" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ServerMessage::GetBat {
                                    args: parseable_args.to_vec(),
                                },
                            )
                            .await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }

                            if server_response_rx.lock().await.changed().await.is_ok() {
                                let value = server_response_rx.lock().await.borrow().clone()?;

                                Ok(Some(value))
                            } else {
                                Err(Arc::from(ServerError::ChannelSend {
                                    message: ServerMessage::GetBat {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "memory" | "mem" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ServerMessage::GetMem {
                                    args: parseable_args.to_vec(),
                                },
                            )
                            .await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }

                            if server_response_rx.lock().await.changed().await.is_ok() {
                                let value = server_response_rx.lock().await.borrow().clone()?;

                                Ok(Some(value))
                            } else {
                                Err(Arc::from(ServerError::ChannelSend {
                                    message: ServerMessage::GetMem {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
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
            "listen" => {
                while listener_rx.lock().await.changed().await.is_ok() {
                    let value = listener_rx.lock().await.borrow().clone();

                    if let Err(e) = socket_write(socket.clone(), value.as_bytes()).await {
                        if let Err(e) = mpsc_send(error_tx.clone(), e).await {
                            eprintln!("Could not send error via channel: {e}");
                        }
                    }
                }

                Ok(None)
            }
            "update" => {
                match args.get(1) {
                    Some(argument) => match argument.as_str() {
                        "volume" | "vol" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ServerMessage::UpdateVol).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "brightness" | "bri" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ServerMessage::UpdateBri).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "battery" | "bat" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ServerMessage::UpdateBat).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "memory" | "mem" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ServerMessage::UpdateMem).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
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

                if server_tx.send(ServerMessage::UpdateAll).await.is_err() {
                    return Err(Arc::from(ServerError::ChannelSend {
                        message: ServerMessage::UpdateAll.to_string(),
                    }));
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

    let (server_tx, mut server_rx) = mpsc::channel(32);
    let (server_response_tx, server_response_rx) = watch::channel(Ok(String::from("")));
    let (listener_tx, listener_rx) = watch::channel(String::from(""));
    let server_tx_1 = server_tx.clone();

    let (error_tx, mut error_rx) = mpsc::channel::<Arc<ServerError>>(32);
    let error_tx_clone = error_tx.clone();

    tokio::spawn(async move {
        while let Some(value) = error_rx.recv().await {
            eprintln!("{value}");
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(e) = mpsc_send(server_tx_1.clone(), ServerMessage::UpdateAll).await {
                eprintln!("Failed to send update message: {e}");
            }

            std::thread::sleep(std::time::Duration::from_millis(1500));
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

    tokio::spawn(async move {
        socket_loop(
            listener,
            error_tx_clone.clone(),
            server_tx.clone(),
            Arc::from(Mutex::new(server_response_rx)),
            Arc::from(Mutex::new(listener_rx)),
        )
        .await;
    });

    while let Some(val) = server_rx.recv().await {
        match val {
            ServerMessage::UpdateAll => {
                if let Err(e) = Battery::update(&bat_mutex).await {
                    eprintln!("{e}");
                }

                if let Err(e) = Memory::update(&mem_mutex).await {
                    eprintln!("{e}");
                }

                match get_all_json(
                    vol_mutex.clone(),
                    bri_mutex.clone(),
                    bat_mutex.clone(),
                    mem_mutex.clone(),
                )
                .await
                {
                    Ok(json) => {
                        if listener_tx.send(json.clone()).is_err() {
                            if let Err(e) = error_tx
                                .send(Arc::from(ServerError::ChannelSend { message: json }))
                                .await
                            {
                                eprintln!("Could not send error via channel: {e}");
                            }
                        }
                    }
                    Err(e) => eprintln!("{e}"),
                };
            }
            ServerMessage::GetVol { args } => {
                let volume = Volume::parse_args(&vol_mutex.clone(), args.as_slice()).await;

                if server_response_tx.send(volume.clone()).is_err() {
                    if let Err(e) = error_tx
                        .send(Arc::from(ServerError::ChannelSend {
                            message: format!("{volume:?}"),
                        }))
                        .await
                    {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ServerMessage::UpdateVol => Volume::update(&vol_mutex).await?,
            ServerMessage::GetBri { args } => {
                let brightness = Brightness::parse_args(&bri_mutex.clone(), args.as_slice()).await;

                if server_response_tx.send(brightness.clone()).is_err() {
                    if let Err(e) = error_tx
                        .send(Arc::from(ServerError::ChannelSend {
                            message: format!("{brightness:?}"),
                        }))
                        .await
                    {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ServerMessage::UpdateBri => Brightness::update(&bri_mutex).await?,
            ServerMessage::GetBat { args } => {
                let battery = Battery::parse_args(&bat_mutex.clone(), args.as_slice()).await;

                if server_response_tx.send(battery.clone()).is_err() {
                    if let Err(e) = error_tx
                        .send(Arc::from(ServerError::ChannelSend {
                            message: format!("{battery:?}"),
                        }))
                        .await
                    {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ServerMessage::UpdateBat => Battery::update(&bat_mutex).await?,
            ServerMessage::GetMem { args } => {
                let memory = Memory::parse_args(&mem_mutex.clone(), args.as_slice()).await;

                if server_response_tx.send(memory.clone()).is_err() {
                    if let Err(e) = error_tx
                        .send(Arc::from(ServerError::ChannelSend {
                            message: format!("{memory:?}"),
                        }))
                        .await
                    {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ServerMessage::UpdateMem => Memory::update(&mem_mutex).await?,
        }
    }

    Ok(())
}
