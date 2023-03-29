use std::{env, time::Duration, collections::HashMap};

use nostr_sdk::prelude::*;
use nostr_sdk::blocking::Client;
use repl_rs::{Repl, Command, Value, Result, Parameter, Convert};

struct Context {
    pub client: Client,
    pub pub_key: XOnlyPublicKey,
}

// #[tokio::main]
fn main() {
    // export SECRET_KEY=$(cat ~/.nostr/key)
    
    let secret = env::var("SECRET_KEY").expect("Secret key not set");

    let my_keys = Keys::from_sk_str(secret.as_str()).unwrap();
    // let my_keys = Keys::generate();

    let pub_key = my_keys.public_key();
    println!("Bech32 PubKey: {}", pub_key.to_bech32().unwrap());
    // println!("PubKey: {}", pub_key.to_string());

    let client = Client::new(&my_keys);
    // client.add_relay("wss://relay.damus.io", None).await?;
    client.add_relay("ws://nostr.extrabits.io", None).unwrap();
    // client.add_relay("ws://localhost:8080", None).unwrap();
    
    client.connect();

    let ctx = Context {
        client,
        pub_key,
    };

    let mut repl = Repl::new(ctx)
        .with_name("nostr-test")
        .with_description("Nostr CLI client")
        .add_command(
            Command::new("puts", puts)
                .with_parameter(Parameter::new("message").set_required(true).unwrap()).unwrap()
                .with_parameter(Parameter::new("kind").set_default("1").unwrap()).unwrap()
                .with_parameter(Parameter::new("title").set_default("").unwrap()).unwrap(),
        )
        .add_command(
            Command::new("gets", gets)
                .with_parameter(Parameter::new("id").set_required(true).unwrap()).unwrap()
        )
        .add_command(
            Command::new("ls", ls)
                .with_parameter(Parameter::new("minutes").set_default("60").unwrap()).unwrap()
        )
        .add_command(
            Command::new("quit", quit)
        );

    println!("Nostr CLI client");
    repl.run().unwrap();
}

fn gets(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let id = args["id"].to_string();

    match context.client.get_events_of(vec!(Filter::new().id(id)), Some(Duration::from_secs(5))) {
        Ok(event) => Ok(Some(format!("{:?}", event.get(0)))),
        Err(error) => Ok(Some(error.to_string())),
    }
    // println!("{:?}", event.get(0));
}

fn ls(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let minutes: u64 = args["minutes"].convert()?;

    let filter = Filter::new()
        .author(context.pub_key)
        .kind(Kind::TextNote)
        .since(Timestamp::now() - Duration::from_secs(60 * minutes));
    let events = context.client.get_events_of(vec!(filter), Some(Duration::from_secs(5))).unwrap();

    println!("Found {} events", events.len());
    for event in events.iter() {
        println!("{:?}", event);

        // client.delete_event(event.id, Some("Just test data")).await?;
    }

    Ok(Some(format!("Getting messages for the last {} minutes", minutes)))
}

fn puts(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let msg = args["message"].to_string();
    let kind: i32 = args["kind"].convert().unwrap_or(1);
    let title = args["title"].to_string();

    let mut tags = Vec::new();
    if title.len() > 0 {
        tags.push(Tag::Title(title))
    }

    match kind {
        1 => match context.client.publish_text_note(msg, &tags) {
            Ok(event_id) => Ok(Some(format!("Just sent event ID {}", event_id))),
            Err(error) => Ok(Some(error.to_string())),
        },
        30023 => match context.client.send_event(EventBuilder::long_form_text_note(msg, &tags).to_event(&context.client.keys()).unwrap()) {
            Ok(event_id) => Ok(Some(format!("Just sent event ID {}", event_id))),
            Err(error) => Ok(Some(error.to_string())),
        },
        _ => Ok(Some("Event kind not supported".into()))
    }

    // let builder = EventBuilder::long_form_text_note("# heading  body  [link](https://extrabits.io)", &[]);
    // let event_id = client.send_event(builder.to_event(&my_keys)?).await?;
}

fn quit(_args: HashMap<String, Value>, _context: &mut Context) -> Result<Option<String>> {
    panic!("quitting")
}