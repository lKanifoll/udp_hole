use clap::Parser;
use std::io;
use std::net::Ipv4Addr;
//use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::spawn;
use tokio::sync::Mutex;

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

const UDPPORT: u16 = 4200;
const HTTPPORT: u16 = 8080;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    println!(
        "Hi, {}! Connecting to {} through {} with timeout {}. Client mode: {}",
        args.my_name, args.peer_name, args.server, args.timeout, args.client
    );

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let socket = Arc::new(Mutex::new(socket));
    let socket_send = Arc::clone(&socket);
    let socket_recv = Arc::clone(&socket);

    // Register to server
    let mut byte_vec = args.my_name.as_bytes().to_vec();

    byte_vec.insert(0, 0x00);
    byte_vec.push(0xFF);
    let socket = socket.lock().await;
    socket.send_to(&byte_vec, (args.server, UDPPORT)).await?;
    //-----------------------

    // Get peer IP
    let get_peer_info_url = format!(
        "http://{}:{}/api/wait/{}",
        args.server, HTTPPORT, args.peer_name
    );
    let params = [("timeout", args.timeout.to_string())];

    let client = reqwest::Client::new();
    let response = client.get(get_peer_info_url).query(&params).send().await?;

    if response.status().is_success() {
        let peer_ip = response.text().await?;
        println!("Peers IP: {}", peer_ip);

        //socket.send_to(&[0], &peer_ip).await?;

        let send_task = spawn(async move {
            loop {
                // let stdin = io::stdin();
                // let mut message = String::new();
                // println!("Enter your message to {}: ", args.peer_name);
                // stdin.read_line(&mut message).unwrap();

                let message = "Привет!";

                let _socket_send = socket_send.lock().await;
                println!("{}", message);
                match _socket_send.send_to(message.as_bytes(), &peer_ip).await {
                    Ok(len) => {
                        println!("Sent {} bytes", len);
                    }
                    Err(e) => {
                        println!("Error receiving: {}", e);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let recv_task = spawn(async move {
            loop {
                let mut buffer = [0; 512];
                let _socket_recv = socket_recv.lock().await;

                match _socket_recv.recv_from(&mut buffer).await {
                    Ok((len, peer_ip)) => {
                        let message = String::from_utf8_lossy(&buffer[..len]);
                        println!("Received from {}: {}", peer_ip, message);
                    }
                    Err(e) => {
                        println!("Error receiving: {}", e);
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
