use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use anyhow::Context;

const TCP_LISTENER_PORT: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ds_listener =
        TcpListener::bind(TCP_LISTENER_PORT).await.context("Failed to open TCP listener server")?;
    println!("Spawned ds_listner server at port {}", TCP_LISTENER_PORT);

    tokio::spawn(tcp_listener(ds_listener));

    let mut stream = TcpStream::connect(TCP_LISTENER_PORT).await?;

    let message = b"HII!!!!";

    loop {
        stream
            .write_all(message)
            .await
            .context("failed to write to tcp stream")?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn tcp_listener(listener: TcpListener) -> anyhow::Result<()> {
    loop {
        let (mut socket, addr) = listener
            .accept()
            .await
            .context("Unable to retrieve socket and address from tcp listener")?;

        println!("connection started from address {}", addr);

        tokio::spawn(async move {
            loop {
                let mut buf = [0; 1024];

                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read: {e}");
                        return;
                    }
                };

                println!("recieved {} bytes from {}", n, addr);

                for i in 0..=n {
                    if i != n {
                        print!("{:#X} ", buf[i]);
                    } else {
                        println!("{:#X} ", buf[i]);
                    }
                }
            }
        });
    }
}
