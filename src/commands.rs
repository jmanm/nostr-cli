use std::time::Duration;

use crate::Context;
use chrono::DateTime;
use nostr_sdk::prelude::*;

#[derive(Debug)]
pub struct PublishArgs {
    message: String,
    kind: Option<i32>,
    title: Option<String>,
    publish_date: Option<String>,
    image_url: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // Cp {
    //     file_name:
    // },
    Gets {
        id: String,
    },
    Ls {
        limit: Option<usize>,
    },
    Puts(PublishArgs),
    Exit,
}

// pub fn cp(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
//     let file_name = args["file"].to_string();

//     match fs::read_to_string(file_name) {
//         Ok(msg) => internal_send_event(msg, args, context),
//         Err(error) => Err(Error::IllegalRequiredError(error.to_string()))
//     }
// }

// pub fn rm(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
//     let id = args["id"].to_string();
//     let event_id = EventId::from_bech32(&id).unwrap();

//     match context.client.delete_event(event_id, Some("Deleted by author")) {
//         Ok(id) => Ok(Some(format!("Deleted event {}", id))),
//         Err(error) => Ok(Some(error.to_string())),
//     }
// }

fn format_event(event: &Event) -> String {
    format!("Event ID: {}\nCreated: {}\nMessage: {}",
        event.id.to_bech32().unwrap(),
        event.created_at,
        String::from_iter(event.content.chars().take(100))
    )
}

pub async fn gets(id: String, context: &mut Context) -> Result<()> {
    let event_id = EventId::from_bech32(&id)?;
    let events = context.client.fetch_events(Filter::new().id(event_id), Duration::from_secs(5)).await?;
    match events.first() {
        Some(event) => println!("{}", format_event(event)),
        None => println!("Event not found"),
    }
    Ok(())
}

pub async fn ls(limit: Option<usize>, context: &mut Context) -> Result<()> {
    let limit = limit.unwrap_or(10);
    println!("Getting the last {} messages", limit);

    let filter = Filter::new()
        .author(context.pub_key)
        .limit(limit);
    let events = context.client.fetch_events(filter, Duration::from_secs(5)).await?;

    println!("Found {} events", events.len());
    for event in events.iter() {
        println!("{}\n", format_event(event));
    }

    Ok(())
}

pub async fn puts(args: PublishArgs, context: &mut Context) -> Result<()> {
    internal_send_event(args, context).await?;
}

async fn internal_send_event(args: PublishArgs, context: &mut Context) -> Result<()> {
    let kind = args.kind.unwrap_or(1);

    let mut tags = Vec::new();
    if let Some(title) = args.title {
        tags.push(Tag::title(title));
    }

    if let Some(publish_date) = args.publish_date {
        // cp /home/jamin/src/extrabits/posts/post.md 30023 "The Next Web" "2022-06-30T19:32:00-08:00" "images/wires.jpg"
        // cp /home/jamin/src/extrabits/posts/nostr.md 30023 "Powered By Nostr" "2023-04-13T20:23:00-07:00" "images/power-lines.jpg"
        let date_time = DateTime::parse_from_rfc3339(&publish_date)?;
        tags.push(Tag::custom(TagKind::PublishedAt, vec![date_time.timestamp().to_string()]));
    }

    if let Some(image_url) = args.image_url {
        let url = Url::parse(&image_url)?;
        tags.push(Tag::image(url, None));
    }

    match kind {
        1 => {
            let evt = EventBuilder::text_note(args.message)
                .tags(tags)
                .build(context.pub_key)
                .sign_with_keys(context.keys)?;
            context.client.se(msg, &tags).await?
        }
        30023 => {
            let evt = EventBuilder::long_form_text_note(msg)
                .tags(tags)
                .to_event(&context.client.keys());
            let event_id = context.client.send_event(evt).await?;
            println!("Just sent event ID {}", event_id);
        }
        _ => println!("Event kind not supported".into())
    }
}
