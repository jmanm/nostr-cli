use std::{fs, time::Duration};

use crate::Context;
use chrono::DateTime;
use clap::{ArgAction, Args, Subcommand};
use nostr_sdk::prelude::*;
use nostr_sdk::nips::nip65::extract_owned_relay_list;

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
    Bcast {
        message: String,
    },
    Cp {
        file_name: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        publish_date: Option<String>,
        #[arg(short, long)]
        image_url: Option<String>,
    },
    Dm {
        pubkey: String,
        message: String,
    },
    Fol {
        pubkey: String,
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
    Relay {
      address: String,
      #[arg(short, long, action = ArgAction::SetTrue)]
      delete: bool,
    },
    Rm {
        id: String,
    },
    Exit,
}

pub async fn handle_command(command: Commands, context: &mut Context) -> Result<()> {
    match command {
        Commands::Bcast { message } =>
            bcast(message, context).await,
        Commands::Cp { file_name, title, publish_date, image_url } =>
            cp(file_name, title, publish_date, image_url, context).await,
        Commands::Dm { pubkey, message } =>
            dm(pubkey, message, context).await,
        Commands::Fol { pubkey } =>
            fol(pubkey, context).await,
        Commands::Gets { id } =>
            gets(id, context).await,
        Commands::Ls { limit } =>
            ls(limit, context).await,
        Commands::Puts { message } =>
            puts(message, context).await,
        Commands::Relay { address, delete } =>
            relay(address, delete, context).await,
        Commands::Rm { id } =>
            rm(id, context).await,
        _ => Ok(()),
    }
}

async fn bcast(message: String, context: &mut Context) -> Result<()> {
    let contacts = context.client.get_contact_list(Duration::from_secs(5)).await?;
    for contact in contacts {
        println!("Sending message to {} ({})", contact.alias.unwrap_or("Unknown alias".into()), contact.public_key);
        context.client.send_private_msg(contact.public_key, &message, vec![]).await?;
    }
    Ok(())
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

async fn dm(pubkey: String, message: String, context: &mut Context) -> Result<()> {
    let public_key = PublicKey::parse(&pubkey)?;
    println!("Sending message to {}", public_key.to_string());
    let result = context.client.send_private_msg(public_key, &message, vec![]).await?;
    println!("Just sent message ID {}", result.id());
    Ok(())
}

async fn fol(pubkey: String, context: &mut Context) -> Result<()> {
    let public_key = PublicKey::parse(&pubkey)?;
    let mut contacts = context.client.get_contact_list(Duration::from_secs(30)).await?;
    contacts.push(Contact {
        public_key,
        relay_url: None,
        alias: None,
    });
    let evt = EventBuilder::contact_list(contacts);
    context.client.send_event_builder(evt).await?;
    println!("Added pubkey {} to contacts", public_key);
    Ok(())
}

fn print_relay_list(list: &Vec<(RelayUrl, Option<RelayMetadata>)>) -> String {
    list.iter().map(|(url, md)| {
        let mut result = url.to_string();
        if let Some(md) = md {
            result.push_str(&format!(" ({})", md));
        }
        result
    }).collect::<Vec<String>>().join("\n")
}

async fn relay(address: String, delete: bool, context: &mut Context) ->  Result<()> {
    match RelayUrl::parse(&address) {
        Ok(url) => {
            let filter = Filter::new()
                .author(context.keys.public_key)
                .kind(Kind::RelayList)
                .limit(1);
            let relay_events = context.client.fetch_events(filter, Duration::from_secs(5)).await?;
            let mut relay_list = if relay_events.len() > 0 {
                extract_owned_relay_list(relay_events.first_owned().unwrap()).collect()
            } else {
                Vec::new()
            };
            if delete {
                if let Some(ix) = relay_list.iter().position(|u| (*u).0 == url) {
                    context.client.remove_relay(&url).await?;
                    relay_list.remove(ix);
                    let relay_list_str = print_relay_list(&relay_list);
                    let evt = EventBuilder::relay_list(relay_list);
                    context.client.send_event_builder(evt).await?;
                    println!("Removed relay {}\nNew relay list:\n{}", &url, relay_list_str);
                } else {
                    println!("Relay was not in user relay list")
                }
            } else {
                context.client.add_relay(&url).await?;
                relay_list.push((url.clone(), None));
                let relay_list_str = print_relay_list(&relay_list);
                let evt = EventBuilder::relay_list(relay_list);
                context.client.send_event_builder(evt).await?;
                println!("Added relay {}\nNew relay list:\n{}", &url, relay_list_str);
            }
        }
        Err(e) => {
            println!("Invalid relay URL:  {}", e);
        }
    }
    Ok(())
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
    let mut lines = vec![
        format!("Event ID: {}", event.id.to_bech32().unwrap()),
        format!("Event Kind: {:?}", event.kind),
        format!("Created: {}", event.created_at.to_human_datetime()),
    ];
    if !event.tags.is_empty() {
        lines.push("Tags".into());
        for tag in event.tags.iter() {
            lines.push(format!("Kind: {}; content: {}", tag.kind(), tag.content().unwrap_or_default()));
        }
    }
    let text_content = String::from_iter(event.content.chars().take(100));
    if !text_content.is_empty() {
        lines.push(format!("Message: {}", text_content));
    }
    return lines.join("\n");
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
    let mut sorted = Vec::from_iter(events);
    sorted.sort_by(|e1, e2| e1.created_at.cmp(&e2.created_at));
    for event in sorted.iter() {
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
