use clap::Parser;
use std::io;
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;

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
        "Hi, {}! Connecting to {} through {} with timeout {}. Client mode: {}",
        args.my_name, args.peer_name, args.server, args.timeout, args.client
    );

    let socket = UdpSocket::bind("0.0.0.0:0").await?;

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
    // Get peer IP
    let response = client.get(get_peer_info_url).query(&params).send().await?;

    if response.status().is_success() {
        let peer_ip = response.text().await?;
        println!("Peers IP: {}", peer_ip);

        socket.send_to(&[0], &peer_ip).await?;

        loop {
            if args.client {
                let stdin = io::stdin();
                let mut message = String::new();
                println!("Enter your message to {}: ", args.peer_name);
                stdin.read_line(&mut message).unwrap();
                socket.send_to(message.as_bytes(), &peer_ip).await?;
            } else {
                let mut buffer = [0; 512];
                let (length, src) = socket.recv_from(&mut buffer).await?;
                let received_data = String::from_utf8_lossy(&buffer[..length]);
                println!("Received from {}: {}", src, received_data);
            }
        }
    } else {
        println!("Error: {}", response.status());
    }

    Ok(())
}
