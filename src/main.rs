use std::{env, time::Duration, collections::HashMap, fs};

use chrono::DateTime;
use nostr_sdk::prelude::*;
use nostr_sdk::blocking::Client;
use repl_rs::{Repl, Command, Error, Value, Result, Parameter, Convert};

struct Context {
    pub client: Client,
    pub pub_key: XOnlyPublicKey,
}

// #[tokio::main]
fn main() {
    // export SECRET_KEY=$(cat ~/.nostr/key)
    
    let my_keys = match env::var("SECRET_KEY") {
        Ok(secret) => {
            println!("Reading secret key from environment");
            Keys::from_sk_str(secret.as_str()).unwrap()
        }
        _ => {
            println!("No secret key specified; generating new keys");
            Keys::generate()
        }
    };

    let pub_key = my_keys.public_key();
    println!("Bech32 PubKey: {}", pub_key.to_bech32().unwrap());

    let client = Client::new(&my_keys);
    // client.add_relay("wss://relay.damus.io", None).await?;
    client.add_relay("ws://nostr.extrabits.io", None).unwrap();
    // client.add_relay("ws://localhost:50001", None).unwrap();
    
    client.connect();

    let ctx = Context {
        client,
        pub_key,
    };

    let mut repl = Repl::new(ctx)
        .with_name("nostr-test")
        .with_description("Nostr CLI client")
        .add_command(
            add_tag_params(
                Command::new("puts", puts)
                    .with_parameter(Parameter::new("message").set_required(true).unwrap()).unwrap())
        )
        .add_command(
            add_tag_params(
                Command::new("cp", cp)
                    .with_parameter(Parameter::new("file").set_required(true).unwrap()).unwrap())
        )
        .add_command(
            add_id_param(Command::new("rm", rm))
        )
        .add_command(
            add_id_param(Command::new("gets", gets))
        )
        .add_command(
            Command::new("ls", ls)
                .with_parameter(Parameter::new("limit").set_default("10").unwrap()).unwrap()
        )
        .add_command(
            Command::new("quit", quit)
        );

    println!("Nostr CLI client");
    repl.run().unwrap();
}

fn cp(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let file_name = args["file"].to_string();
    
    match fs::read_to_string(file_name) {
        Ok(msg) => internal_send_event(msg, args, context),
        Err(error) => Err(Error::IllegalRequiredError(error.to_string()))
    }
}

fn rm(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let id = args["id"].to_string();
    let event_id = EventId::from_bech32(id).unwrap();

    match context.client.delete_event(event_id, Some("Deleted by author")) {
        Ok(id) => Ok(Some(format!("Deleted event {}", id))),
        Err(error) => Ok(Some(error.to_string())),
    }
}

fn format_event(event: &Event) -> String {
    format!("Event ID: {}\nCreated: {}\nMessage: {}",
        event.id.to_bech32().unwrap(),
        event.created_at,
        String::from_iter(event.content.chars().take(100))
    )
}

fn gets(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let id = args["id"].to_string();
    if let Ok(event_id) = EventId::from_bech32(id) {
        return match context.client.get_events_of(vec!(Filter::new().id(event_id)), Some(Duration::from_secs(5))) {
            Ok(events) => {
                match events.get(0) {
                    Some(event) => Ok(Some(format!("{}", format_event(event)))),
                    None => Ok(Some("Event not found".to_string())),
                }
            },
            Err(error) => Ok(Some(error.to_string())),
        }
    }

    Err(Error::IllegalRequiredError("Invalid Id".into()))
}

fn ls(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let limit: usize = args["limit"].convert()?;

    let filter = Filter::new()
        .author(context.pub_key)
        .limit(limit);
    let events = context.client.get_events_of(vec!(filter), Some(Duration::from_secs(5))).unwrap();

    println!("Found {} events", events.len());
    for event in events.iter() {
        println!("{}\n", format_event(event));
    }

    Ok(Some(format!("Getting the last {} messages", limit)))
}

fn puts(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let msg = args["message"].to_string();
    internal_send_event(msg, args, context)
}

fn internal_send_event(msg: String, args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let kind: i32 = args["kind"].convert().unwrap_or(1);
    let title = args["title"].to_string();
    let publish_date = args["published date"].to_string();
    let image_url = args["image url"].to_string();

    let mut tags = Vec::new();
    if title.len() > 0 {
        tags.push(Tag::Title(title))
    }

    if publish_date.len() > 0 {
        // cp /home/jamin/src/extrabits/posts/post.md 30023 "The Next Web" "2022-06-30T19:32:00-08:00" "images/wires.jpg"
        // cp /home/jamin/src/extrabits/posts/nostr.md 30023 "Powered By Nostr" "2023-04-13T20:23:00-07:00" "images/power-lines.jpg"
        match DateTime::parse_from_rfc3339(&publish_date) {
            Ok(date_time) => tags.push(Tag::PublishedAt(Timestamp::from(date_time.timestamp() as u64))),
            Err(error) => return Ok(Some(error.to_string()))
        }
    }

    if image_url.len() > 0 {
        tags.push(Tag::Image(image_url));
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
}

fn quit(_args: HashMap<String, Value>, _context: &mut Context) -> Result<Option<String>> {
    panic!("quitting")
}

fn add_tag_params(cmd: Command<Context, Error>) -> Command<Context, Error> {
    cmd.with_parameter(Parameter::new("kind").set_default("1").unwrap()).unwrap()
        .with_parameter(Parameter::new("title").set_default("").unwrap()).unwrap()
        .with_parameter(Parameter::new("published date").set_default("").unwrap()).unwrap()
        .with_parameter(Parameter::new("image url").set_default("").unwrap()).unwrap()
}

fn add_id_param(cmd: Command<Context, Error>) -> Command<Context, Error> {
    cmd.with_parameter(Parameter::new("id").set_required(true).unwrap()).unwrap()
}