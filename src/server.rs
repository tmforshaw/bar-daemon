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

type ServerResult<T> = Result<T, Arc<ServerError>>;

#[derive(Clone)]
enum ChannelCommand {
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

impl std::fmt::Display for ChannelCommand {
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
    server_tx: mpsc::Sender<ChannelCommand>,
    server_response_rx: Arc<Mutex<watch::Receiver<ServerResult<String>>>>,
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
    server_tx: mpsc::Sender<ChannelCommand>,
    server_response_rx: Arc<Mutex<watch::Receiver<ServerResult<String>>>>,
    listener_rx: Arc<Mutex<watch::Receiver<String>>>,
) -> ServerResult<Option<String>> {
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
                                ChannelCommand::GetVol {
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
                                    message: ChannelCommand::GetVol {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "brightness" | "bri" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ChannelCommand::GetBri {
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
                                    message: ChannelCommand::GetBri {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "battery" | "bat" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ChannelCommand::GetBat {
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
                                    message: ChannelCommand::GetBat {
                                        args: parseable_args.to_vec(),
                                    }
                                    .to_string(),
                                }))
                            }
                        }
                        "memory" | "mem" => {
                            if let Err(e) = mpsc_send(
                                server_tx,
                                ChannelCommand::GetMem {
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
                                    message: ChannelCommand::GetMem {
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

                        // Exit out of loop when socket disconnects
                        return Ok(None);
                    }
                }

                Ok(None)
            }
            "update" => {
                match args.get(1) {
                    Some(argument) => match argument.as_str() {
                        "volume" | "vol" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ChannelCommand::UpdateVol).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "brightness" | "bri" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ChannelCommand::UpdateBri).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "battery" | "bat" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ChannelCommand::UpdateBat).await
                            {
                                if let Err(e) = mpsc_send(error_tx, e).await {
                                    eprintln!("Could not send error via channel: {e}");
                                }
                            }
                        }
                        "memory" | "mem" => {
                            if let Err(e) =
                                mpsc_send(server_tx.clone(), ChannelCommand::UpdateMem).await
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

                if server_tx.send(ChannelCommand::UpdateAll).await.is_err() {
                    return Err(Arc::from(ServerError::ChannelSend {
                        message: ChannelCommand::UpdateAll.to_string(),
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

pub async fn start() -> ServerResult<()> {
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

    let (error_tx, mut error_rx) = mpsc::channel::<Arc<ServerError>>(32);

    tokio::spawn(async move {
        while let Some(value) = error_rx.recv().await {
            eprintln!("{value}");
        }
    });

    let server_tx_1 = server_tx.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = mpsc_send(server_tx_1.clone(), ChannelCommand::UpdateAll).await {
                eprintln!("Failed to send update message: {e}");
            }

            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    });

    let vol_mutex = Arc::new(Mutex::new(match call_and_retry(Volume::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => return Err(e),
        None => return Err(Arc::from(ServerError::RetryError)),
    }));

    let bri_mutex = Arc::new(Mutex::new(
        match call_and_retry(Brightness::get_json_tuple) {
            Some(Ok(out)) => out,
            Some(Err(e)) => return Err(e),
            None => return Err(Arc::from(ServerError::RetryError)),
        },
    ));

    let bat_mutex = Arc::new(Mutex::new(match call_and_retry(Battery::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => return Err(e),
        None => return Err(Arc::from(ServerError::RetryError)),
    }));

    let mem_mutex = Arc::new(Mutex::new(match call_and_retry(Memory::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => return Err(e),
        None => return Err(Arc::from(ServerError::RetryError)),
    }));

    let error_tx_1 = error_tx.clone();

    tokio::spawn(async move {
        socket_loop(
            listener,
            error_tx_1,
            server_tx.clone(),
            Arc::from(Mutex::new(server_response_rx)),
            Arc::from(Mutex::new(listener_rx)),
        )
        .await;
    });

    while let Some(val) = server_rx.recv().await {
        match val {
            ChannelCommand::UpdateAll => {
                if let Err(e) = Battery::update(&bat_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }

                if let Err(e) = Memory::update(&mem_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
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
            ChannelCommand::GetVol { args } => {
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
            ChannelCommand::UpdateVol => {
                if let Err(e) = Volume::update(&vol_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ChannelCommand::GetBri { args } => {
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
            ChannelCommand::UpdateBri => {
                if let Err(e) = Brightness::update(&bri_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ChannelCommand::GetBat { args } => {
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
            ChannelCommand::UpdateBat => {
                if let Err(e) = Battery::update(&bat_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ChannelCommand::GetMem { args } => {
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
            ChannelCommand::UpdateMem => {
                if let Err(e) = Memory::update(&mem_mutex).await {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}
