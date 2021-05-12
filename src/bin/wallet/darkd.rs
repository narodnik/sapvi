#[macro_use]
extern crate clap;
use async_executor::Executor;
use async_native_tls::TlsAcceptor;
use async_std::sync::Mutex;
use easy_parallel::Parallel;
use ff::Field;
use http_types::{Request, Response, StatusCode};
use log::*;
use rand::rngs::OsRng;
use rusqlite::Connection;
use sapvi::serial;
use serde_json::json;
use smol::Async;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::sync::Arc;

use sapvi::{net, Error, Result};

/// Listens for incoming connections and serves them.
async fn listen(
    executor: Arc<Executor<'_>>,
    rpc: Arc<RpcInterface>,
    listener: Async<TcpListener>,
    tls: Option<TlsAcceptor>,
) -> Result<()> {
    // Format the full host address.
    let host = match &tls {
        None => format!("http://{}", listener.get_ref().local_addr()?),
        Some(_) => format!("https://{}", listener.get_ref().local_addr()?),
    };
    println!("Listening on {}", host);

    loop {
        // Accept the next connection.
        let (stream, _) = listener.accept().await?;

        // Spawn a background task serving this connection.
        let task = match &tls {
            None => {
                let stream = async_dup::Arc::new(stream);
                let rpc = rpc.clone();
                executor.spawn(async move {
                    if let Err(err) = async_h1::accept(stream, move |req| {
                        let rpc = rpc.clone();
                        rpc.serve(req)
                    })
                    .await
                    {
                        println!("Connection error: {:#?}", err);
                    }
                })
            }
            Some(tls) => {
                // In case of HTTPS, establish a secure TLS connection first.
                match tls.accept(stream).await {
                    Ok(stream) => {
                        let _stream = async_dup::Arc::new(async_dup::Mutex::new(stream));
                        executor.spawn(async move {
                            /*if let Err(err) = async_h1::accept(stream, serve).await {
                                println!("Connection error: {:#?}", err);
                            }*/
                            unimplemented!();
                        })
                    }
                    Err(err) => {
                        println!("Failed to establish secure TLS connection: {:#?}", err);
                        continue;
                    }
                }
            }
        };

        // Detach the task to let it run in the background.
        task.detach();
    }
}

struct RpcInterface {
    p2p: Arc<net::P2p>,
    started: Mutex<bool>,
    stop_send: async_channel::Sender<()>,
    stop_recv: async_channel::Receiver<()>,
}

impl RpcInterface {
    fn new(p2p: Arc<net::P2p>) -> Arc<Self> {
        let (stop_send, stop_recv) = async_channel::unbounded::<()>();

        Arc::new(Self {
            p2p,
            started: Mutex::new(false),
            stop_send,
            stop_recv,
        })
    }

    async fn db_connect() -> Connection {
        let path = dirs::home_dir()
            .expect("Cannot find home directory.")
            .as_path()
            .join(".config/darkfi/wallet.db");
        let connector = Connection::open(&path);
        connector.expect("Failed to connect to database.")
    }

    async fn generate_key() -> (Vec<u8>, Vec<u8>) {
        let secret: jubjub::Fr = jubjub::Fr::random(&mut OsRng);
        let public = zcash_primitives::constants::SPENDING_KEY_GENERATOR * secret;
        let pubkey = serial::serialize(&public);
        let privkey = serial::serialize(&secret);
        (privkey, pubkey)
    }

    // TODO: fix this
    async fn store_key(conn: &Connection, pubkey: Vec<u8>, privkey: Vec<u8>) -> Result<()> {
        let mut db_file = File::open("wallet.sql")?;
        let mut contents = String::new();
        db_file.read_to_string(&mut contents)?;
        Ok(conn.execute_batch(&mut contents)?)
    }

    // add new methods to handle wallet commands
    async fn serve(self: Arc<Self>, mut req: Request) -> http_types::Result<Response> {
        info!("RPC serving {}", req.url());

        let request = req.body_string().await?;

        let mut io = jsonrpc_core::IoHandler::new();
        io.add_sync_method("say_hello", |_| {
            Ok(jsonrpc_core::Value::String("Hello World!".into()))
        });

        let self2 = self.clone();
        io.add_method("get_info", move |_| {
            let self2 = self2.clone();
            async move {
                Ok(json!({
                    "started": *self2.started.lock().await,
                    "connections": self2.p2p.connections_count().await
                }))
            }
        });

        let stop_send = self.stop_send.clone();
        io.add_method("stop", move |_| {
            let stop_send = stop_send.clone();
            async move {
                let _ = stop_send.send(()).await;
                Ok(jsonrpc_core::Value::Null)
            }
        });
        io.add_method("key_gen", move |_| async move {
            RpcInterface::db_connect().await;
            let (pubkey, privkey) = RpcInterface::generate_key().await;
            //println!("{}", pubkey, "{}", privkey);
            Ok(jsonrpc_core::Value::Null)
        });

        let response = io
            .handle_request_sync(&request)
            .ok_or(sapvi::Error::BadOperationType)?;

        let mut res = Response::new(StatusCode::Ok);
        res.insert_header("Content-Type", "text/plain");
        res.set_body(response);
        Ok(res)
    }

    async fn wait_for_quit(self: Arc<Self>) -> Result<()> {
        Ok(self.stop_recv.recv().await?)
    }
}

async fn start(executor: Arc<Executor<'_>>, options: ProgramOptions) -> Result<()> {
    let p2p = net::P2p::new(options.network_settings);

    let rpc = RpcInterface::new(p2p.clone());
    let http = listen(
        executor.clone(),
        rpc.clone(),
        Async::<TcpListener>::bind(([127, 0, 0, 1], options.rpc_port))?,
        None,
    );

    let http_task = executor.spawn(http);

    *rpc.started.lock().await = true;

    p2p.clone().start(executor.clone()).await?;

    p2p.run(executor).await?;

    rpc.wait_for_quit().await?;

    http_task.cancel().await;

    Ok(())
}

/*
async fn start2(executor: Arc<Executor<'_>>, options: ProgramOptions) -> Result<()> {
    let connections = Arc::new(Mutex::new(HashMap::new()));

    let stored_addrs = Arc::new(Mutex::new(Vec::new()));

    let executor2 = executor.clone();
    let stored_addrs2 = stored_addrs.clone();

    let mut server_task = None;
    if let Some(accept_addr) = options.accept_addr {
        let accept_addr = accept_addr.clone();

        let protocol = ServerProtocol::new(connections.clone(), accept_addr, stored_addrs2);
        server_task = Some(executor.spawn(async move {
            protocol.start(executor2).await?;
            Ok::<(), sapvi::Error>(())
        }));
    }

    let mut seed_protocols = Vec::with_capacity(options.seed_addrs.len());

    // Normally we query this from a server
    let accept_addr = options.accept_addr.clone();

    for seed_addr in options.seed_addrs.iter() {
        let protocol = SeedProtocol::new(seed_addr.clone(), accept_addr, stored_addrs.clone());
        protocol.clone().start(executor.clone()).await;
        seed_protocols.push(protocol);
    }

    debug!("Waiting for seed node queries to finish...");

    for seed_protocol in seed_protocols {
        seed_protocol.await_finish().await;
    }

    debug!("Seed nodes queried.");

    let mut client_slots = vec![];
    for i in 0..options.connection_slots {
        debug!("Starting connection slot {}", i);

        let client = Channel::new(
            connections.clone(),
            accept_addr.clone(),
            stored_addrs.clone(),
        );
        client.clone().start(executor.clone()).await;
        client_slots.push(client);
    }

    for remote_addr in options.manual_connects {
        debug!("Starting connection (manual) to {}", remote_addr);

        let client = Channel::new(
            connections.clone(),
            accept_addr.clone(),
            stored_addrs.clone(),
        );
        client
            .clone()
            .start_manual(remote_addr, executor.clone())
            .await;
        client_slots.push(client);
    }

    let rpc = RpcInterface::new();
    let http = listen(
        executor.clone(),
        rpc.clone(),
        Async::<TcpListener>::bind(([127, 0, 0, 1], 8000))?,
        None,
    );

    let http_task = executor.spawn(http);

    rpc.stop_recv.recv().await?;

    http_task.cancel().await;

    match server_task {
        None => {}
        Some(server_task) => {
            server_task.cancel().await;
        }
    }
    Ok(())
}
*/

struct ProgramOptions {
    network_settings: net::Settings,
    log_path: Box<std::path::PathBuf>,
    rpc_port: u16,
}

impl ProgramOptions {
    fn load() -> Result<ProgramOptions> {
        let app = clap_app!(dfi =>
            (version: "0.1.0")
            (author: "Amir Taaki <amir@dyne.org>")
            (about: "Dark node")
            (@arg ACCEPT: -a --accept +takes_value "Accept address")
            (@arg SEED_NODES: -s --seeds ... "Seed nodes")
            (@arg CONNECTS: -c --connect ... "Manual connections")
            (@arg CONNECT_SLOTS: --slots +takes_value "Connection slots")
            (@arg LOG_PATH: --log +takes_value "Logfile path")
            (@arg RPC_PORT: -r --rpc +takes_value "RPC port")
        )
        .get_matches();

        let accept_addr = if let Some(accept_addr) = app.value_of("ACCEPT") {
            Some(accept_addr.parse()?)
        } else {
            None
        };

        let mut seed_addrs: Vec<SocketAddr> = vec![];
        if let Some(seeds) = app.values_of("SEED_NODES") {
            for seed in seeds {
                seed_addrs.push(seed.parse()?);
            }
        }

        let mut manual_connects: Vec<SocketAddr> = vec![];
        if let Some(connections) = app.values_of("CONNECTS") {
            for connect in connections {
                manual_connects.push(connect.parse()?);
            }
        }

        let connection_slots = if let Some(connection_slots) = app.value_of("CONNECT_SLOTS") {
            connection_slots.parse()?
        } else {
            0
        };

        let log_path = Box::new(
            if let Some(log_path) = app.value_of("LOG_PATH") {
                std::path::Path::new(log_path)
            } else {
                std::path::Path::new("/tmp/darkfid.log")
            }
            .to_path_buf(),
        );

        let rpc_port = if let Some(rpc_port) = app.value_of("RPC_PORT") {
            rpc_port.parse()?
        } else {
            8000
        };

        Ok(ProgramOptions {
            network_settings: net::Settings {
                inbound: accept_addr,
                outbound_connections: connection_slots,
                external_addr: accept_addr,
                peers: manual_connects,
                seeds: seed_addrs,
                ..Default::default()
            },
            log_path,
            rpc_port,
        })
    }
}

fn main() -> Result<()> {
    use simplelog::*;

    let options = ProgramOptions::load()?;

    let logger_config = ConfigBuilder::new().set_time_format_str("%T%.6f").build();

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, logger_config, TerminalMode::Mixed).unwrap(),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            std::fs::File::create(options.log_path.as_path()).unwrap(),
        ),
    ])
    .unwrap();

    let ex = Arc::new(Executor::new());
    let (signal, shutdown) = async_channel::unbounded::<()>();
    let ex2 = ex.clone();

    let (_, result) = Parallel::new()
        // Run four executor threads.
        .each(0..3, |_| smol::future::block_on(ex.run(shutdown.recv())))
        // Run the main future on the current thread.
        .finish(|| {
            smol::future::block_on(async move {
                start(ex2, options).await?;
                drop(signal);
                Ok::<(), sapvi::Error>(())
            })
        });

    result
}
