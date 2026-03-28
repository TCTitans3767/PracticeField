use std::net::{TcpListener, TcpStream};

use anyhow::Context;

const TCP_LISTENER_PORT: &str = "10.0.100.5";

fn main() -> anyhow::Result<()>{
    
    let ds_listener = TcpListener::bind("10.0.100.5").context("Failed to open TCP listener server")?;
    println!("Spawned ds_listner server at port {}", TCP_LISTENER_PORT);

    Ok(())
}
