use std::{fs, time::Duration};

use crate::Context;
use chrono::DateTime;
use clap::{Args, Subcommand};
use nostr_sdk::prelude::*;

#[derive(Debug, Args)]
pub struct PublishArgs {
    message: String,
    kind: Kind,
    title: Option<String>,
    publish_date: Option<String>,
    image_url: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Cp {
        file_name: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        publish_date: Option<String>,
        #[arg(short, long)]
        image_url: Option<String>,
    },
    Gets {
        id: String,
    },
    Ls {
        limit: Option<usize>,
    },
    Puts {
        message: String,
    },
    Rm {
        id: String,
    },
    Exit,
}

pub async fn handle_command(command: Commands, context: &mut Context) -> Result<()> {
    match command {
        Commands::Cp { file_name, title, publish_date, image_url } =>
            cp(file_name, title, publish_date, image_url, context).await,
        Commands::Gets { id } =>
            gets(id, context).await,
        Commands::Ls { limit } =>
            ls(limit, context).await,
        Commands::Puts { message } =>
            puts(message, context).await,
        Commands::Rm { id } =>
            rm(id, context).await,
        _ => Ok(()),
    }
}

async fn cp(
    file_name: String,
    title: Option<String>,
    publish_date: Option<String>,
    image_url: Option<String>,
    context: &mut Context
) -> Result<()> {
    let message = fs::read_to_string(file_name)?;
    let args = PublishArgs {
        message,
        kind: Kind::LongFormTextNote,
        title,
        publish_date,
        image_url,
    };
    internal_send_event(args, context).await
}

async fn rm(id: String, context: &mut Context) -> Result<()> {
    let event_id = EventId::from_bech32(&id).unwrap();
    let evt = EventBuilder::delete(EventDeletionRequest {
        ids: vec![event_id],
        coordinates: vec![],
        reason: Some("Deleted by author".to_string()),
    });
    context.client.send_event_builder(evt).await?;
    println!("Deleted event {}", event_id);
    Ok(())
}

fn format_event(event: &Event) -> String {
    format!("Event ID: {}\nCreated: {}\nMessage: {}",
        event.id.to_bech32().unwrap(),
        event.created_at,
        String::from_iter(event.content.chars().take(100))
    )
}

async fn gets(id: String, context: &mut Context) -> Result<()> {
    let event_id = EventId::from_bech32(&id)?;
    let events = context.client.fetch_events(Filter::new().id(event_id), Duration::from_secs(5)).await?;
    match events.first() {
        Some(event) => println!("{}", format_event(event)),
        None => println!("Event not found"),
    }
    Ok(())
}

async fn ls(limit: Option<usize>, context: &mut Context) -> Result<()> {
    let limit = limit.unwrap_or(10);
    println!("Getting the last {} messages", limit);

    let filter = Filter::new()
        .author(context.keys.public_key)
        .limit(limit);
    let events = context.client.fetch_events(filter, Duration::from_secs(5)).await?;

    println!("Found {} events", events.len());
    for event in events.iter() {
        println!("{}\n", format_event(event));
    }

    Ok(())
}

async fn puts(message: String, context: &mut Context) -> Result<()> {
    let args = PublishArgs {
        message,
        kind: Kind::TextNote,
        title: None,
        publish_date: None,
        image_url: None,
    };
    internal_send_event(args, context).await
}

async fn internal_send_event(args: PublishArgs, context: &mut Context) -> Result<()> {
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

    match args.kind {
        Kind::TextNote => {
            let evt = EventBuilder::text_note(args.message).tags(tags);
            let result = context.client.send_event_builder(evt).await?;
            println!("Just sent event ID {}", result.id());
        }
        Kind::LongFormTextNote => {
            let evt = EventBuilder::long_form_text_note(args.message).tags(tags);
            let result = context.client.send_event_builder(evt).await?;
            println!("Just sent event ID {}", result.id());
        }
        _ => println!("Event kind {} not supported", args.kind)
    }
    Ok(())
}
