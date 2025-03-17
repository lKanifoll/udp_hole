use clap::Parser;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::spawn;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "Alex")]
    my_name: String,
    #[arg(long, default_value = "Sergey")]
    peer_name: String,
    #[arg(long, default_value = "45.151.30.139")]
    server: Ipv4Addr,
    #[arg(long, default_value = "10")]
    timeout: u8,
    #[arg(long, default_value = "false")]
    client: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    println!(
        "Hi, {}! Connecting to {} through {} with timeout {}.",
        args.my_name, args.peer_name, args.server, args.timeout
    );

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let socket = Arc::new(socket);
    let socket_send = Arc::clone(&socket);
    let socket_recv = Arc::clone(&socket);

    // Register to server
    let mut byte_vec = args.my_name.as_bytes().to_vec();

    byte_vec.insert(0, 0x00);
    byte_vec.push(0xFF);

    socket.send_to(&byte_vec, (args.server, 4200)).await?;
    //-----------------------

    // Get peer IP
    let get_peer_info_url = format!(
        "http://{}:{}/api/wait/{}",
        args.server, 8080, args.peer_name
    );
    let params = [("timeout", args.timeout.to_string())];

    let client = reqwest::Client::new();
    let response = client.get(get_peer_info_url).query(&params).send().await?;

    if response.status().is_success() {
        let peer_ip = response.text().await?;
        println!("Peers IP: {}", peer_ip);

        let send_task = spawn(async move {
            loop {
                let stdin = io::stdin();
                let mut message = String::new();
                println!("Enter your message to {}: ", &args.peer_name);
                stdin.read_line(&mut message).unwrap();

                match socket_send.send_to(message.as_bytes(), &peer_ip).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to send: {}", e);
                    }
                }
            }
        });

        let recv_task = spawn(async move {
            loop {
                let mut buffer = [0; 512];

                match socket_recv.recv_from(&mut buffer).await {
                    Ok((len, _peer_ip)) => {
                        let message = String::from_utf8_lossy(&buffer[..len]);
                        println!("Received from: {}", message);
                    }
                    Err(e) => {
                        println!("Failed to receive: {}", e);
                    }
                }
            }
        });

        let _ = tokio::join!(send_task, recv_task);
    } else {
        println!("Error: {}", response.status());
    }
    //------------

    Ok(())
}
