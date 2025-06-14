use std::io::Write;
use std::str::FromStr;
use std::env;

use nostr_sdk::{Client, Keys, PublicKey, ToBech32};
use clap::{Parser, Subcommand};

mod commands;

struct Context {
    pub client: Client,
    pub pub_key: PublicKey,
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

    let my_keys = match env::var("SECRET_KEY") {
        Ok(secret) => {
            println!("Reading secret key from environment");
            Keys::from_str(secret.as_str()).unwrap()
        }
        _ => {
            println!("No secret key specified; generating new keys");
            Keys::generate()
        }
    };

    let pub_key = my_keys.public_key();
    println!("Bech32 PubKey: {}", pub_key.to_bech32().unwrap());

    let client = Client::new(my_keys);
    // client.add_relay("wss://relay.damus.io").await?;
    // client.add_relay("wss://nostr.extrabits.io").unwrap();
    client.add_relay("ws://localhost:5001").await.map_err(|e| e.to_string())?;
    client.connect().await;

    let mut ctx = Context {
        client,
        pub_key,
    };

    // let mut repl = Repl::new(ctx)
    //     .with_name("nostr-test")
    //     .with_description("Nostr CLI client")
    //     .add_command(
    //         add_tag_params(
    //             Command::new("puts", puts)
    //                 .with_parameter(Parameter::new("message").set_required(true).unwrap()).unwrap())
    //     )
    //     .add_command(
    //         add_tag_params(
    //             Command::new("cp", cp)
    //                 .with_parameter(Parameter::new("file").set_required(true).unwrap()).unwrap())
    //     )
    //     .add_command(
    //         add_id_param(Command::new("rm", rm))
    //     )

    println!("Nostr CLI client");
    loop {
        let line = readline()?;
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
    match cli.command {
        commands::Commands::Gets { id } =>
            commands::gets(id, context).await.map_err(|e| e.to_string())?,
        commands::Commands::Ls { limit } =>
            commands::ls(limit, context).await.map_err(|e| e.to_string())?,
        commands::Commands::Exit => {
            println!("Exiting ...");
            return Ok(true);
        }
    }
    Ok(false)
}

// fn add_tag_params(cmd: Command<Context, Error>) -> Command<Context, Error> {
//     cmd.with_parameter(Parameter::new("kind").set_default("1").unwrap()).unwrap()
//         .with_parameter(Parameter::new("title").set_default("").unwrap()).unwrap()
//         .with_parameter(Parameter::new("published date").set_default("").unwrap()).unwrap()
//         .with_parameter(Parameter::new("image url").set_default("").unwrap()).unwrap()
// }

// fn add_id_param(cmd: Command<Context, Error>) -> Command<Context, Error> {
//     cmd.with_parameter(Parameter::new("id").set_required(true).unwrap()).unwrap()
// }

fn readline() -> Result<String, String> {
    write!(std::io::stdout(), "$ ").map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buffer = String::new();
    std::io::stdin()
        .read_line(&mut buffer)
        .map_err(|e| e.to_string())?;
    Ok(buffer)
}
