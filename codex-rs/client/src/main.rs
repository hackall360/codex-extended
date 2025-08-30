use std::net::SocketAddr;

use clap::Parser;
use codex_core::protocol::Submission;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;

#[derive(Debug, Parser)]
struct ClientCli {
    #[arg(long, default_value = "127.0.0.1:7878")]
    connect: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = ClientCli::parse();
    let stream = TcpStream::connect(cli.connect).await?;
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half).lines();

    // Task to forward stdin to the server
    let write_task = tokio::spawn(async move {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Submission>(&line) {
                Ok(sub) => {
                    let s = serde_json::to_string(&sub).unwrap();
                    if write_half.write_all(s.as_bytes()).await.is_err() {
                        break;
                    }
                    if write_half.write_all(b"\n").await.is_err() {
                        break;
                    }
                    if write_half.flush().await.is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("invalid submission: {e}"),
            }
        }
    });

    // Task to print events from the server
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            println!("{}", line);
        }
    });

    let _ = tokio::join!(write_task, read_task);
    Ok(())
}
