#![allow(clippy::too_many_lines)]
use crate::battery::Battery;
use crate::bluetooth::Bluetooth;
use crate::brightness::Brightness;
use crate::channel::{mpsc_send, send_and_await_response, ServerCommand};
use crate::command::{
    call_and_retry_async, get_all_json, get_tup, send_or_print_err, socket_read, socket_write,
    ServerError, ServerResult,
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
            send_or_print_err(e, &error_tx).await;

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
                send_or_print_err(Arc::from(ServerError::AddressInUse { e }), &error_tx).await;

                return;
            }
        };

        let server_tx_1 = server_tx.clone();
        let server_response_rx_1 = server_response_rx.clone();
        let listener_rx_1 = listener_rx.clone();
        let error_tx_1 = error_tx.clone();

        tokio::spawn(async move {
            let Some(args) = process_socket_message(socket.clone(), error_tx_1.clone()).await
            else {
                return;
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
                        send_or_print_err(e, &error_tx_1).await;
                    }
                    return;
                }
            };

            if let Some(r) = reply {
                if let Err(e) = socket_write(socket, r.as_bytes()).await {
                    send_or_print_err(e, &error_tx_1).await;
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
                    valid: ["volume", "brightness", "battery", "memory", "bluetooth"]
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
            send_or_print_err(e, &error_tx).await;

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
    if let Some(argument) = args.get(1) {
        match argument.as_str() {
            "volume" | "vol" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateVol).await {
                    send_or_print_err(e, &error_tx).await;
                }
            }
            "brightness" | "bri" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBri).await {
                    send_or_print_err(e, &error_tx).await;
                }
            }
            "battery" | "bat" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBat).await {
                    send_or_print_err(e, &error_tx).await;
                }
            }
            "memory" | "mem" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateMem).await {
                    send_or_print_err(e, &error_tx).await;
                }
            }
            "bluetooth" | "bt" => {
                if let Err(e) = mpsc_send(server_tx.clone(), ServerCommand::UpdateBlu).await {
                    send_or_print_err(e, &error_tx).await;
                }
            }
            incorrect => {
                return Err(Arc::from(ServerError::IncorrectArgument {
                    incorrect: incorrect.to_string(),
                    valid: ["volume", "brightness", "battery", "memory", "bluetooth"]
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect(),
                }))
            }
        }
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
    if let Some(command) = args.first() {
        match command.as_str() {
            "get" => parse_get_args(args, server_tx, server_response_rx, error_tx).await,
            "listen" => parse_listen_args(listener_rx, socket, error_tx).await,
            "update" => parse_update_args(args, server_tx, error_tx).await,
            incorrect => Err(Arc::from(ServerError::IncorrectArgument {
                incorrect: incorrect.to_string(),
                valid: ["get", "listen", "update"]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            })),
        }
    } else {
        Err(Arc::from(ServerError::EmptyArguments))
    }
}

async fn update_all(
    vol_tup: &[(String, String)],
    bri_tup: &[(String, String)],
    bat_tup: &mut Vec<(String, String)>,
    mem_tup: &mut Vec<(String, String)>,
    blu_tup: &[(String, String)],
    listener_tx: &watch::Sender<String>,
    error_tx: &mpsc::Sender<Arc<ServerError>>,
) {
    update_battery(bat_tup, error_tx).await;
    update_memory(mem_tup, error_tx).await;

    match get_all_json(vol_tup, bri_tup, bat_tup, mem_tup, blu_tup) {
        Ok(json) => {
            if listener_tx.send(json.clone()).is_err() {
                send_or_print_err(
                    Arc::from(ServerError::ChannelSend { message: json }),
                    error_tx,
                )
                .await;
            }
        }
        Err(e) => eprintln!("{e}"),
    };
}

async fn update_battery(
    bat_tup: &mut Vec<(String, String)>,
    error_tx: &mpsc::Sender<Arc<ServerError>>,
) {
    match Battery::update(bat_tup) {
        Ok(tup) => *bat_tup = tup,
        Err(e) => {
            send_or_print_err(e, error_tx).await;
        }
    }
}

async fn update_memory(
    mem_tup: &mut Vec<(String, String)>,
    error_tx: &mpsc::Sender<Arc<ServerError>>,
) {
    match Memory::update() {
        Ok(tup) => *mem_tup = tup,
        Err(e) => {
            send_or_print_err(e, error_tx).await;
        }
    }
}

async fn server_channel_loop(
    server_rx: &mut mpsc::Receiver<ServerCommand>,
    server_response_tx: watch::Sender<ServerResult<String>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    listener_tx: watch::Sender<String>,
) {
    let (
        Some(mut vol_tup),
        Some(mut bri_tup),
        Some(mut bat_tup),
        Some(mut mem_tup),
        Some(mut blu_tup),
    ) = (
        get_tup(Volume::get_json_tuple, &error_tx).await,
        get_tup(Brightness::get_json_tuple, &error_tx).await,
        get_tup(Battery::get_json_tuple, &error_tx).await,
        get_tup(Memory::get_json_tuple, &error_tx).await,
        get_tup(Bluetooth::get_json_tuple, &error_tx).await,
    )
    else {
        return;
    };

    while let Some(val) = server_rx.recv().await {
        match val {
            ServerCommand::UpdateAll => {
                update_all(
                    &vol_tup,
                    &bri_tup,
                    &mut bat_tup,
                    &mut mem_tup,
                    &blu_tup,
                    &listener_tx,
                    &error_tx,
                )
                .await;
            }
            ServerCommand::GetVol { args } => {
                let volume = Volume::parse_args(&vol_tup, args.as_slice());

                println!("Found Volume {volume:?}");

                if server_response_tx.send(volume.clone()).is_err() {
                    send_or_print_err(
                        Arc::from(ServerError::ChannelSend {
                            message: format!("{volume:?}"),
                        }),
                        &error_tx,
                    )
                    .await;
                }
            }
            ServerCommand::UpdateVol => match Volume::update() {
                Ok(tup) => vol_tup = tup,
                Err(e) => {
                    send_or_print_err(e, &error_tx).await;
                }
            },
            ServerCommand::GetBri { args } => {
                let brightness = Brightness::parse_args(&bri_tup, args.as_slice());

                if server_response_tx.send(brightness.clone()).is_err() {
                    send_or_print_err(
                        Arc::from(ServerError::ChannelSend {
                            message: format!("{brightness:?}"),
                        }),
                        &error_tx,
                    )
                    .await;
                }
            }
            ServerCommand::UpdateBri => match Brightness::update() {
                Ok(tup) => bri_tup = tup,
                Err(e) => {
                    send_or_print_err(e, &error_tx).await;
                }
            },
            ServerCommand::GetBat { args } => {
                let battery = Battery::parse_args(&bat_tup, args.as_slice());

                if server_response_tx.send(battery.clone()).is_err() {
                    send_or_print_err(
                        Arc::from(ServerError::ChannelSend {
                            message: format!("{battery:?}"),
                        }),
                        &error_tx,
                    )
                    .await;
                }
            }
            ServerCommand::UpdateBat => update_battery(&mut bat_tup, &error_tx).await,
            ServerCommand::GetMem { args } => {
                let memory = Memory::parse_args(&mem_tup, args.as_slice());

                if server_response_tx.send(memory.clone()).is_err() {
                    send_or_print_err(
                        Arc::from(ServerError::ChannelSend {
                            message: format!("{memory:?}"),
                        }),
                        &error_tx,
                    )
                    .await;
                }
            }
            ServerCommand::UpdateMem => update_memory(&mut mem_tup, &error_tx).await,
            ServerCommand::GetBlu { args } => {
                let bluetooth = Bluetooth::parse_args(&blu_tup, args.as_slice());

                if server_response_tx.send(bluetooth.clone()).is_err() {
                    send_or_print_err(
                        Arc::from(ServerError::ChannelSend {
                            message: format!("{bluetooth:?}"),
                        }),
                        &error_tx,
                    )
                    .await;
                }
            }
            ServerCommand::UpdateBlu => match Bluetooth::update() {
                Ok(tup) => blu_tup = tup,
                Err(e) => {
                    send_or_print_err(e, &error_tx).await;
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
    let (server_response_tx, server_response_rx) = watch::channel(Ok(String::new()));
    let (listener_tx, listener_rx) = watch::channel(String::new());

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
