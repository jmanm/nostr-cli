use std::io::Write;
use std::str::FromStr;
use std::env;

use nostr_sdk::{Client, Keys, ToBech32};
use clap::Parser;

mod commands;

struct Context {
    pub client: Client,
    pub keys: Keys,
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // export SECRET_KEY=$(cat ~/.nostr/key)

    let keys = match env::var("SECRET_KEY") {
        Ok(secret) => {
            println!("Reading secret key from environment");
            Keys::from_str(secret.as_str()).unwrap()
        }
        _ => {
            println!("No secret key specified; generating new keys");
            Keys::generate()
        }
    };

    println!("Bech32 PubKey: {}", keys.public_key.to_bech32().unwrap());

    let client = Client::new(keys.clone());
    // client.add_relay("wss://relay.damus.io").await?;
    // client.add_relay("wss://nostr.extrabits.io").unwrap();
    client.add_relay("ws://localhost:5001").await.map_err(|e| e.to_string())?;
    client.connect().await;

    let mut ctx = Context {
        client,
        keys,
    };


    println!("Nostr CLI client");
    loop {
        let line = readline().map_err(|e| e.to_string())?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match respond(line, &mut ctx).await {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                write!(std::io::stdout(), "{err}").map_err(|e| e.to_string())?;
                std::io::stdout().flush().map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

async fn respond(line: &str, context: &mut Context) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    if let commands::Commands::Exit = cli.command {
        return Ok(true);
    }
    commands::handle_command(cli.command, context)
        .await
        .map_err(|e| e.to_string())?;
    Ok(false)
}

fn readline() -> std::io::Result<String> {
    write!(std::io::stdout(), "$ ")?;
    std::io::stdout().flush()?;
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}
