use std::sync::{Arc, Mutex};
use tracing::{debug, warn, error, info};

use actix_web::{web, App, HttpServer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp, TcpListener, TcpStream, UdpSocket},
};

use anyhow::{Context, Ok};

pub mod driverstation_comms;

use crate::driverstation_comms::{
    fms_commands::init_fms_queue,
    tcp::ds_tcp_listener,
};

const TCP_LISTENER_PORT: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true) // Show module names in logs
        .with_thread_ids(true) // Include thread ID
        .with_line_number(true) // Include source line numbers
        .with_writer(std::io::stderr) // Write to stderr
        .init();

    // Initialize FMS command queue (single global instance)
    let _fms_queue = init_fms_queue();
    debug!("FMS command queue initialized");

    let ds_listener = TcpListener::bind(TCP_LISTENER_PORT)
        .await
        .context("Failed to open TCP listener server")?;

    debug!("Spawned ds_listner server at port {}", TCP_LISTENER_PORT);

    let ds_udp_socket = UdpSocket::bind("0.0.0.0:8080").await?;
    let shared_udp_socket = Arc::new(ds_udp_socket);

    tokio::spawn(tcp_listener(
        ds_listener,
        shared_udp_socket.clone(),
    ));

    // TODO: Test client - uncomment to test with mock driver station
    {
        let mut stream = TcpStream::connect(TCP_LISTENER_PORT).await?;
        let message: [u8; 5] = [0xff, 0xff, 0x18, 0x0e, 0xb7];

        tokio::spawn(async move {
            loop {
                stream
                    .write_all(&message)
                    .await
                    .context("failed to write to tcp stream")
                    .unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    let _ = HttpServer::new(|| App::new().route("/status", web::get().to(interface::status)))
        .bind("127.0.0.1:2000")
        .context("failed to run web server")?
        .run()
        .await;

    Ok(())
}

async fn tcp_listener(
    listener: TcpListener,
    shared_udp_socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .context("Unable to retrieve socket and address from tcp listener")?;

        debug!("connection started from address {}", addr);

        tokio::spawn(ds_tcp_listener(
            socket,
            addr,
            shared_udp_socket.clone(),
        ));
    }
}

pub mod interface {
    use actix_web::{HttpResponse, Responder};

    pub async fn status() -> impl Responder {
        HttpResponse::Ok().body("all good")
    }
}
