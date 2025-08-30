use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::signal;
use tracing::{error, info};

/// Connect to a remote Codex server over TCP and proxy stdin/stdout.
#[derive(Debug, Parser)]
pub struct ClientCli {
    /// Address of the Codex server
    #[clap(long, default_value = "127.0.0.1:3030")]
    addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = ClientCli::parse();
    run_main(cli).await
}

async fn run_main(opts: ClientCli) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let stream = TcpStream::connect(&opts.addr).await?;
    info!("connected to {}", &opts.addr);
    let (read_half, mut write_half) = stream.into_split();
    let reader = BufReader::new(read_half);

    let sq = async {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        loop {
            let line = tokio::select! {
                _ = signal::ctrl_c() => break,
                res = lines.next_line() => res,
            };
            match line {
                Ok(Some(line)) => {
                    if write_half.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                    if write_half.write_all(b"\n").await.is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    };

    let eq = async move {
        let mut lines = reader.lines();
        loop {
            let line = tokio::select! {
                _ = signal::ctrl_c() => break,
                res = lines.next_line() => res,
            };
            match line {
                Ok(Some(line)) => println!("{line}"),
                Ok(None) => break,
                Err(e) => {
                    error!("{e:#}");
                    break;
                }
            }
        }
    };

    tokio::join!(sq, eq);
    Ok(())
}
