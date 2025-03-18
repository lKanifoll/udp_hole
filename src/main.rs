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
}

async fn registration(arg: &Cli, socket: &UdpSocket) {
    let mut byte_vec = arg.my_name.as_bytes().to_vec();
    byte_vec.insert(0, 0x00);
    byte_vec.push(0xFF);
    match socket.send_to(&byte_vec, (arg.server, 4200)).await {
        Ok(_) => {}
        Err(e) => panic!("Failed to send registration: {e:?}"),
    }
}

async fn get_peer_ip(arg: &Cli, server: &Ipv4Addr) -> Result<String, reqwest::Error> {
    let get_peer_info_url = format!("http://{}:{}/api/wait/{}", server, 8080, arg.peer_name);
    let params = [("timeout", arg.timeout.to_string())];

    let client = reqwest::Client::new();
    let response = client.get(get_peer_info_url).query(&params).send().await?;

    if response.status().is_success() {
        return response.text().await;
    } else {
        panic!("Error: {}", response.status());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    println!(
        "\nHi, {}! Connecting to {} through {} with timeout {}.\n",
        args.my_name, args.peer_name, args.server, args.timeout
    );

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let socket = Arc::new(socket);

    registration(&args, &socket).await;

    let peer_ip = match get_peer_ip(&args, &args.server).await {
        Ok(ip) => ip,
        Err(e) => panic!("Failed to get peer IP: {}", e),
    };
    //println!("Peers IP: {}", peer_ip.as_str());

    // Sending task
    println!("Enter your message to {}: ", args.peer_name);
    let socket_send = Arc::clone(&socket);
    let send_task = spawn(async move {
        loop {
            let stdin = io::stdin();
            let mut message = String::new();

            stdin.read_line(&mut message).unwrap();

            let message = message.trim();

            if !message.is_empty() {
                match socket_send
                    .send_to(message.as_bytes(), peer_ip.as_str())
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to send: {}", e);
                    }
                }
            } else {
                println!("Message is empty. Wrote something.");
            }
        }
    });

    // Receiving task
    let socket_recv = Arc::clone(&socket);
    let recv_task = spawn(async move {
        loop {
            let mut buffer = [0; 512];

            match socket_recv.recv_from(&mut buffer).await {
                Ok((len, _peer_ip)) => {
                    let message = String::from_utf8_lossy(&buffer[..len]);
                    println!("-->: {}", message);
                }
                Err(e) => {
                    println!("Failed to receive: {}", e);
                }
            }
        }
    });

    let _ = tokio::join!(send_task, recv_task);

    Ok(())
}
