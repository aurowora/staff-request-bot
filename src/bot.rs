use std::collections::HashSet;

use serenity::client::EventHandler;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{DispatchError, Args, HelpOptions, CommandGroup, CommandResult, help_commands};
use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::utils::{ArgumentConvert, Colour};

use crate::db::{RequestChannelPair, write_channel_pair, delete_by_requests, read_channel_pair};
use crate::MongoConn;

pub struct Bot;

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        let mongo_client_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<MongoConn>().expect("Expected MongoConn in Data").clone()
        };
    
        let channel_pair = {
            let mongo_client = mongo_client_lock.read().await;
            read_channel_pair(&mongo_client, &msg.channel_id.to_string()).await
        };

        if channel_pair.is_none() {return;}
        // Past this point, we do have a channel pair for this message

        // If it's a thread created message, delete it instead of reacting
        if msg.kind == MessageType::ThreadCreated {
            let _ = msg.delete(&ctx);
        } else {
            let _ = msg.react(&ctx, '✅').await;
            let _ = msg.react(&ctx, '❌').await;
        }
    }

    async fn reaction_add(&self, ctx: Context, rxn: Reaction) {
        // Don't hit the DB at all if it isn't either of the reacts we care about
        if !rxn.emoji.unicode_eq("✅") && !rxn.emoji.unicode_eq("❌") {return;}

        let current_user = match ctx.http.get_current_user().await {
            Ok(user) => user,
            Err(_) => {return;}
        };

        // These will be nice to have later
        let rxn_member = match rxn.member {
            Some(member) => member,
            None => {return;}
        };

        let rxn_user = match &rxn_member.user {
            Some(user) => user,
            None => {return;}
        };

        // Ignore bot's own reacts
        if rxn_user.id == current_user.id {return;}

        // Fetch the channel pair from the database if it exists
        let mongo_client_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<MongoConn>().expect("Expected MongoConn in Data").clone()
        };
    
        let channel_pair = {
            let mongo_client = mongo_client_lock.read().await;
            read_channel_pair(&mongo_client, &rxn.channel_id.to_string()).await
        };

        if channel_pair.is_none() {
            return;
        }
        // We do actually have to do something past this point
        let channel_pair = channel_pair.unwrap();

        let archive_id = match str::parse::<u64>(&channel_pair.archive_channel) {
            Ok(archive_id) => ChannelId(archive_id),
            Err(_) => {
                return;
            }
        };

        // Fetch the actual message
        let message = match ctx.http.get_message(*rxn.channel_id.as_u64(), *rxn.message_id.as_u64()).await {
            Ok(message) => message,
            Err(_) => {
                return;
            }
        };

        // Check if the member reacting is a part of the manager group

        let mut in_role = rxn_user.id == message.author.id; // Alternatively, users may mark their own requests as completed / unnecessary
        for role_id in rxn_member.roles {
            if role_id.to_string() == channel_pair.manager_role {
                in_role = true;
                break;
            }
        }

        if !in_role {
            return;
        }

        // Archive an associated thread if there's anything there
        let message_flags = match message.flags {
            Some(flags) => flags,
            None => {
                return;
            }
        };

        if message_flags & MessageFlags::HAS_THREAD == MessageFlags::HAS_THREAD {
            let thread_id = ChannelId(*message.id.as_u64()); // threads have the same ids as the message they are based on
            let _ = thread_id.edit_thread(&ctx, |e| {
                e.archived(true)
            }).await;
        }

        //Send our message to the archive channel
        let _ = archive_id.send_message(&ctx, |m| {
            m.embed(|e| {
                e.title({
                    if rxn.emoji.unicode_eq("✅") {
                        "Request Complete"
                    } else {
                        "Request Discarded"
                    }
                })
                .colour({
                    if rxn.emoji.unicode_eq("✅") {
                        Colour::new(0x9bdb4d)
                    } else {
                        Colour::new(0xed5353)
                    }
                })
                .description(&message.content)
                .author(|a| {
                    a.name(format!("{}#{:0>4}", message.author.name, message.author.discriminator))
                    .icon_url({
                        // Serenity gives us a size 1024 avatar, which ends up blurring most avatars
                        // size 24 happens to look perfect
                        if message.author.avatar.is_none() {
                            message.author.default_avatar_url()
                        } else {
                            format!("https://cdn.discordapp.com/avatars/{}/{}.png?size=24", message.author.id.0, message.author.avatar.unwrap())
                        }
                    })
                })
                .field("Closed By", &format!("{}", rxn_user), true)
                .field("Thread", {
                    if message_flags & MessageFlags::HAS_THREAD == MessageFlags::HAS_THREAD {
                        format!("{}", ChannelId(*message.id.as_u64()).mention())
                    } else {
                        "None".to_string()
                    }
                }, true)
                .timestamp(message.timestamp)
            })
        }).await;

        // Bug in serenity prevents message.delete() from working even when it should, so this is my work around
        let _ = rxn.channel_id.delete_message(&ctx, message.id).await;
    }
}

// Regular bot stuff

#[hook]
pub async fn unknown_command(ctx: &Context, msg: &Message, command: &str) {
    let _ = msg.reply(ctx, format!("I don't know a `{}` command.", command)).await;
}

#[hook]
pub async fn bot_error(ctx: &Context, msg: &Message, error: DispatchError) {
    if let DispatchError::LackingPermissions(_) = error {
        let _ = msg.reply(&ctx, "You do not have permission to do this.").await;
    }
}

#[help]
async fn help_command(
   context: &Context,
   msg: &Message,
   args: Args,
   help_options: &'static HelpOptions,
   groups: &[&'static CommandGroup],
   owners: HashSet<UserId>
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

// The actual command stuff
#[group]
#[commands(init_requests, destroy_requests)]
struct General;

#[command("setupBoard")]

#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]

#[description("Creates a new request board")]
#[usage("[request_channel] [archive_channel] [manager_role]")]
#[example("#requests #archive @Staff")]
async fn init_requests(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let requests_channel = GuildChannel::convert(&ctx, None, None, &args.single::<String>()?).await?;
    let archive_channel = GuildChannel::convert(&ctx, None, None, &args.single::<String>()?).await?;
    let manager_role = Role::convert(&ctx, msg.guild_id, None, &args.single::<String>()?).await?;

    if requests_channel.kind != ChannelType::Text || archive_channel.kind != ChannelType::Text {
        msg.reply(&ctx, "Both channels must be text channels within guilds in which the bot is present").await?;
        return Ok(());
    }

    let request_channel_pair = RequestChannelPair {
        requests_channel: requests_channel.id.to_string(),
        archive_channel: archive_channel.id.to_string(),
        manager_role: manager_role.id.to_string()
    };

    let mongo_client_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoConn>().expect("Expected MongoConn in Data").clone()
    };

    let err = {
        let mongo_client = mongo_client_lock.read().await;
        write_channel_pair(&mongo_client, &request_channel_pair).await
    };

    if err.is_none() {
        msg.reply(&ctx, format!("Successfully created a request board in {} with {} as the archive channel.", requests_channel, archive_channel)).await?;
    } else {
        msg.reply(&ctx, format!("Failed to create a request board in {} with {} as the archive channel.", requests_channel, archive_channel)).await?;
    }

    Ok(())
}

#[command("removeBoard")]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]

#[description("Destroys an existing request board")]
#[usage("[request_channel]")]
#[example("#requests")]
async fn destroy_requests(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let requests_channel = GuildChannel::convert(&ctx, None, None, &args.single::<String>()?).await?;

    let mongo_client_lock = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoConn>().expect("Expected MongoConn in Data").clone()
    };

    let err = {
        let mongo_client = mongo_client_lock.read().await;
        delete_by_requests(&mongo_client, &requests_channel.id.to_string()).await
    };

    if err.is_none() {
        msg.reply(&ctx, format!("Destroyed the request board associated with {}.", requests_channel)).await?;
    } else {
        msg.reply(&ctx, format!("Failed to destroy request board associated with {}.", requests_channel)).await?;
    };
    Ok(())
}
