use crate::battery::Battery;
use crate::bluetooth::Bluetooth;
use crate::brightness::Brightness;
use crate::channel::{mpsc_send, send_and_await_response, ServerCommand};
use crate::command::{
    call_and_retry, call_and_retry_async, get_all_json, socket_read, socket_write, ServerError,
    ServerResult,
};
use crate::memory::Memory;
use crate::volume::Volume;

use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch, Mutex};

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
    server_tx: mpsc::Sender<ServerCommand>,
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

async fn parse_get_args(
    args: &[String],
    server_tx: mpsc::Sender<ServerCommand>,
    server_response_rx: Arc<Mutex<watch::Receiver<ServerResult<String>>>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
) -> ServerResult<Option<String>> {
    {
        let parseable_args = if args.len() > 2 {
            args.split_at(2).1
        } else {
            return Err(Arc::from(ServerError::EmptyArguments));
        };

        match args.get(1) {
            Some(argument) => match argument.as_str() {
                "volume" | "vol" => Ok(Some(
                    send_and_await_response(
                        ServerCommand::GetVol {
                            args: parseable_args.to_vec(),
                        },
                        server_tx,
                        server_response_rx,
                        error_tx,
                    )
                    .await?,
                )),
                "brightness" | "bri" => Ok(Some(
                    send_and_await_response(
                        ServerCommand::GetBri {
                            args: parseable_args.to_vec(),
                        },
                        server_tx,
                        server_response_rx,
                        error_tx,
                    )
                    .await?,
                )),
                "battery" | "bat" => Ok(Some(
                    send_and_await_response(
                        ServerCommand::GetBat {
                            args: parseable_args.to_vec(),
                        },
                        server_tx,
                        server_response_rx,
                        error_tx,
                    )
                    .await?,
                )),
                "memory" | "mem" => Ok(Some(
                    send_and_await_response(
                        ServerCommand::GetMem {
                            args: parseable_args.to_vec(),
                        },
                        server_tx,
                        server_response_rx,
                        error_tx,
                    )
                    .await?,
                )),
                "bluetooth" | "bt" => Ok(Some(
                    send_and_await_response(
                        ServerCommand::GetBlu {
                            args: parseable_args.to_vec(),
                        },
                        server_tx,
                        server_response_rx,
                        error_tx,
                    )
                    .await?,
                )),
                incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["volume", "brightness", "battery", "memory", "bluetooth"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                })),
            },
            None => Err(Arc::from(ServerError::EmptyArguments)),
        }
    }
}

async fn parse_listen_args(
    listener_rx: Arc<Mutex<watch::Receiver<String>>>,
    socket: Arc<Mutex<TcpStream>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
) -> ServerResult<Option<String>> {
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

async fn parse_update_args(
    args: &[String],
    server_tx: mpsc::Sender<ServerCommand>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
) -> ServerResult<Option<String>> {
    match args.get(1) {
        Some(argument) => match argument.as_str() {
            "volume" | "vol" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateVol).await {
                    if let Err(e) = mpsc_send(error_tx, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            "brightness" | "bri" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBri).await {
                    if let Err(e) = mpsc_send(error_tx, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            "battery" | "bat" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBat).await {
                    if let Err(e) = mpsc_send(error_tx, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            "memory" | "mem" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateMem).await {
                    if let Err(e) = mpsc_send(error_tx, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            "bluetooth" | "bt" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBlu).await {
                    if let Err(e) = mpsc_send(error_tx, e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            incorrect => {
                return Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: vec!["volume", "brightness", "battery", "memory", "bluetooth"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                }))
            }
        },
        None => {}
    };

    if server_tx.send(ServerCommand::UpdateAll).await.is_err() {
        return Err(Arc::from(ServerError::ChannelSend {
            message: ServerCommand::UpdateAll.to_string(),
        }));
    };

    Ok(None)
}

async fn parse_args(
    socket: Arc<Mutex<TcpStream>>,
    args: &[String],
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerCommand>,
    server_response_rx: Arc<Mutex<watch::Receiver<ServerResult<String>>>>,
    listener_rx: Arc<Mutex<watch::Receiver<String>>>,
) -> ServerResult<Option<String>> {
    match args.get(0) {
        Some(command) => match command.as_str() {
            "get" => parse_get_args(args, server_tx, server_response_rx, error_tx).await,
            "listen" => parse_listen_args(listener_rx, socket, error_tx).await,
            "update" => parse_update_args(args, server_tx, error_tx).await,
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

async fn server_channel_loop(
    server_rx: &mut mpsc::Receiver<ServerCommand>,
    server_response_tx: watch::Sender<ServerResult<String>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    listener_tx: watch::Sender<String>,
) {
    let mut vol_tup = match call_and_retry(Volume::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        None => {
            if let Err(e) = error_tx.send(Arc::from(ServerError::RetryError)).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let mut bri_tup = match call_and_retry(Brightness::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        None => {
            if let Err(e) = error_tx.send(Arc::from(ServerError::RetryError)).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let mut bat_tup = match call_and_retry(Battery::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        None => {
            if let Err(e) = error_tx.send(Arc::from(ServerError::RetryError)).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let mut mem_tup = match call_and_retry(Memory::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        None => {
            if let Err(e) = error_tx.send(Arc::from(ServerError::RetryError)).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let mut blu_tup = match call_and_retry(Bluetooth::get_json_tuple) {
        Some(Ok(out)) => out,
        Some(Err(e)) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        None => {
            if let Err(e) = error_tx.send(Arc::from(ServerError::RetryError)).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    while let Some(val) = server_rx.recv().await {
        match val {
            ServerCommand::UpdateAll => {
                match Battery::update(&mut bat_tup).await {
                    Ok(tup) => mem_tup = tup,
                    Err(e) => {
                        if let Err(e) = error_tx.send(e).await {
                            eprintln!("Could not send error via channel: {e}");
                        }
                    }
                }
                match Memory::update().await {
                    Ok(tup) => mem_tup = tup,
                    Err(e) => {
                        if let Err(e) = error_tx.send(e).await {
                            eprintln!("Could not send error via channel: {e}");
                        }
                    }
                }
                match get_all_json(&vol_tup, &bri_tup, &bat_tup, &mem_tup, &blu_tup) {
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
            ServerCommand::GetVol { args } => {
                let volume = Volume::parse_args(&vol_tup, args.as_slice()).await;

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
            ServerCommand::UpdateVol => match Volume::update().await {
                Ok(tup) => vol_tup = tup,
                Err(e) => {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            },
            ServerCommand::GetBri { args } => {
                let brightness = Brightness::parse_args(&bri_tup, args.as_slice()).await;

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
            ServerCommand::UpdateBri => match Brightness::update().await {
                Ok(tup) => bri_tup = tup,
                Err(e) => {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            },
            ServerCommand::GetBat { args } => {
                let battery = Battery::parse_args(&bat_tup, args.as_slice()).await;

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
            ServerCommand::UpdateBat => match Battery::update(&mut bat_tup).await {
                Ok(tup) => bat_tup = tup,
                Err(e) => {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            },
            ServerCommand::GetMem { args } => {
                let memory = Memory::parse_args(&mem_tup, args.as_slice()).await;

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
            ServerCommand::UpdateMem => match Memory::update().await {
                Ok(tup) => mem_tup = tup,
                Err(e) => {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            },
            ServerCommand::GetBlu { args } => {
                let bluetooth = Bluetooth::parse_args(&blu_tup, args.as_slice()).await;

                if server_response_tx.send(bluetooth.clone()).is_err() {
                    if let Err(e) = error_tx
                        .send(Arc::from(ServerError::ChannelSend {
                            message: format!("{bluetooth:?}"),
                        }))
                        .await
                    {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            }
            ServerCommand::UpdateBlu => match Bluetooth::update().await {
                Ok(tup) => blu_tup = tup,
                Err(e) => {
                    if let Err(e) = error_tx.send(e).await {
                        eprintln!("Could not send error via channel: {e}");
                    }
                }
            },
        }
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

    // Error handling loop
    tokio::spawn(async move {
        while let Some(value) = error_rx.recv().await {
            eprintln!("{value}");
        }
    });

    // Update sending loop
    let server_tx_1 = server_tx.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = mpsc_send(server_tx_1.clone(), ServerCommand::UpdateAll).await {
                eprintln!("Failed to send update message: {e}");
            }

            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    });

    // Socket loop
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

    // Server channel loop
    server_channel_loop(&mut server_rx, server_response_tx, error_tx, listener_tx).await;

    Ok(())
}
