use std::{io::Write, time::Duration};
use std::str::FromStr;
use std::env;

use nostr_sdk::{Client, Keys, ToBech32};
use clap::Parser;
use rustyline::{error::ReadlineError, DefaultEditor};

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
    client.add_relay("wss://relay.damus.io").await.map_err(|e| e.to_string())?;
    client.add_relay("wss://nostr.land").await.map_err(|e| e.to_string())?;
    client.add_relay("wss://nos.lol").await.map_err(|e| e.to_string())?;
    client.add_relay("wss://nostr.extrabits.io").await.map_err(|e| e.to_string())?;
    // client.add_relay("ws://localhost:5001").await.map_err(|e| e.to_string())?;
    client.connect().await;

    let username = match client.fetch_metadata(keys.public_key, Duration::from_secs(5)).await {
        Ok(Some(md)) => md.display_name.unwrap_or(md.name.unwrap_or("Unknown user".into())),
        Ok(None) => "Unknown user".into(),
        Err(e) => {
            println!("Error retrieving username: {}", e);
            "Unknown user".into()
        }
    };

    let mut ctx = Context {
        client,
        keys,
    };

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;
    if rl.load_history(".nostr-cli-history").is_err() {
        () // noop
    }
    
    println!("Nostr CLI client");
    loop {
        let readline = rl.readline(&format!("{}>", username));

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line).map_err(|e| e.to_string())?;

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
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C detected, exiting...");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D detected, exiting...");
                // This is triggered by Ctrl-D
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
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
