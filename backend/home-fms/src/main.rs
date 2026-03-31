use std::sync::{Arc, Mutex};

use actix_web::{App, HttpServer, web};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket, tcp},
};

use anyhow::{Context, Ok};

pub mod driverstation_comms;

use crate::driverstation_comms::{
    fms::{self, FMS},
    tcp::{ds_tcp_listener, parse_driverstation_tcp},
    udp,
};
use crate::interface::status;

const TCP_LISTENER_PORT: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ds_listener = TcpListener::bind(TCP_LISTENER_PORT)
        .await
        .context("Failed to open TCP listener server")?;
    println!("Spawned ds_listner server at port {}", TCP_LISTENER_PORT);

    let ds_udp_socket = UdpSocket::bind("0.0.0.0:8080").await?;
    let shared_udp_socket = Arc::new(ds_udp_socket);
    let fms = Arc::new(Mutex::new(FMS {
        driverstations: Vec::new(),
    }));

    tokio::spawn(tcp_listener(
        ds_listener,
        shared_udp_socket.clone(),
        fms.clone(),
    ));

    let mut stream = TcpStream::connect(TCP_LISTENER_PORT).await?;

    let message: [u8; 5] = [0xff, 0xff, 0x18, 0x0e, 0xb7];

    tokio::spawn(async move {
        loop {
            stream
                .write_all(&message)
                .await
                .context("failed to write to tcp stream").unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let _ = HttpServer::new(|| App::new().route("/status", web::get().to(status)))
        .bind("127.0.0.1:2000")
        .context("failed to run web server")?
        .run()
        .await;

    Ok(())
}

async fn tcp_listener(
    listener: TcpListener,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>,
) -> anyhow::Result<()> {
    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .context("Unable to retrieve socket and address from tcp listener")?;

        println!("connection started from address {}", addr);

        tokio::spawn(ds_tcp_listener(
            socket,
            addr,
            shared_udp_socket.clone(),
            fms.clone(),
        ));
    }
}

pub mod interface {
    use actix_web::{HttpResponse, Responder};

    pub async fn status() -> impl Responder {
        HttpResponse::Ok().body("all good")
    }
}
