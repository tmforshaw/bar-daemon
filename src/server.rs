use crate::battery::Battery;
use crate::brightness::Brightness;
use crate::command::{call_and_retry, call_and_retry_async, get_all_json, ServerError};
use crate::memory::Memory;
use crate::volume::Volume;

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch, Mutex};

enum ServerMessage {
    Update,
}

impl std::fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Update => "Update server",
            }
        )
    }
}

async fn process_socket_message(
    socket: Arc<Mutex<TcpStream>>,
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerMessage>,
    socket_rx: Arc<Mutex<watch::Receiver<String>>>,
) {
    let mut buf = [0; 1024];

    let n = match socket.lock().await.read(&mut buf).await {
        Ok(n) if n == 0 => {
            if let Err(e) = error_tx
                .send(Arc::from(ServerError::SocketDisconnect))
                .await
            {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
        Ok(n) => n,
        Err(e) => {
            if let Err(e) = error_tx
                .send(Arc::from(ServerError::SocketRead { e }))
                .await
            {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let message = match String::from_utf8(Vec::from(&buf[0..n])) {
        Ok(string) => string,
        Err(e) => {
            if let Err(e) = error_tx
                .send(Arc::from(ServerError::StringConversion {
                    debug_string: format!("{:?}", &buf[0..n]),
                    e,
                }))
                .await
            {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    let args = message
        .split_ascii_whitespace()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();

    let reply = match parse_args(
        socket.clone(),
        &args,
        vol_mutex,
        bri_mutex,
        bat_mutex,
        mem_mutex,
        error_tx.clone(),
        server_tx,
        socket_rx,
    )
    .await
    {
        Ok(reply) => reply,
        Err(e) => {
            if let Err(e) = error_tx.send(e).await {
                eprintln!("Could not send error via channel: {e}");
            }
            return;
        }
    };

    if let Some(r) = reply {
        if let Err(e) = socket.lock().await.write_all(r.as_bytes()).await {
            if let Err(e) = error_tx
                .send(Arc::from(ServerError::SocketWrite { e }))
                .await
            {
                eprintln!("Could not send error via channel: {e}");
            }
        }
    };
}

async fn socket_loop(
    listener: TcpListener,
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerMessage>,
    socket_rx: Arc<Mutex<watch::Receiver<String>>>,
) {
    loop {
        let socket = match listener.accept().await {
            Ok((socket, _)) => Arc::from(Mutex::new(socket)),
            Err(e) => {
                if let Err(e) = error_tx
                    .send(Arc::from(ServerError::AddressInUse { e }))
                    .await
                {
                    eprintln!("Could not send error via channel: {e}");
                };
                return;
            }
        };

        let vol_mutex_1 = vol_mutex.clone();
        let bri_mutex_1 = bri_mutex.clone();
        let bat_mutex_1 = bat_mutex.clone();
        let mem_mutex_1 = mem_mutex.clone();

        let server_tx_1 = server_tx.clone();
        let socket_rx_1 = socket_rx.clone();
        let error_tx_1 = error_tx.clone();

        tokio::spawn(async move {
            process_socket_message(
                socket.clone(),
                vol_mutex_1,
                bri_mutex_1,
                bat_mutex_1,
                mem_mutex_1,
                error_tx_1,
                server_tx_1,
                socket_rx_1,
            )
            .await;
        });
    }
}

async fn parse_args(
    socket: Arc<Mutex<TcpStream>>,
    args: &[String],
    vol_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bri_mutex: Arc<Mutex<Vec<(String, String)>>>,
    bat_mutex: Arc<Mutex<Vec<(String, String)>>>,
    mem_mutex: Arc<Mutex<Vec<(String, String)>>>,
    error_tx: mpsc::Sender<Arc<ServerError>>,
    server_tx: mpsc::Sender<ServerMessage>,
    socket_rx: Arc<Mutex<watch::Receiver<String>>>,
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
            "listen" => {
                while socket_rx.lock().await.changed().await.is_ok() {
                    let value = socket_rx.lock().await.borrow().clone();

                    if socket
                        .lock()
                        .await
                        .write_all(value.as_bytes())
                        .await
                        .is_err()
                    {
                        if let Err(e) = error_tx
                            .send(Arc::from(ServerError::ChannelSend { message: value }))
                            .await
                        {
                            eprintln!("Could not send error via channel: {e}");
                        }
                    }
                }

                Ok(None)
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

                if server_tx.send(ServerMessage::Update).await.is_err() {
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

    let (server_tx, mut server_rx) = mpsc::channel(32);
    let (socket_tx, socket_rx) = watch::channel(String::from("Test"));
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
            if let Err(e) = server_tx_1.send(ServerMessage::Update).await {
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

    let vol_mutex_1 = vol_mutex.clone();
    let bri_mutex_1 = bri_mutex.clone();
    let bat_mutex_1 = bat_mutex.clone();
    let mem_mutex_1 = mem_mutex.clone();

    tokio::spawn(async move {
        socket_loop(
            listener,
            vol_mutex_1.clone(),
            bri_mutex_1.clone(),
            bat_mutex_1.clone(),
            mem_mutex_1.clone(),
            error_tx_clone.clone(),
            server_tx.clone(),
            Arc::from(Mutex::new(socket_rx)),
        )
        .await;
    });

    while let Some(val) = server_rx.recv().await {
        match val {
            ServerMessage::Update => {
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
                        if socket_tx.send(json.clone()).is_err() {
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
        }
    }

    Ok(())
}
