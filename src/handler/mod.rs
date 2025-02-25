use anyhow::anyhow;
use entity::koi;
use itertools::Itertools;
use sea_orm::{entity::*, DatabaseConnection, EntityTrait, InsertResult, QueryFilter};
use serenity::all::{
    ChannelId, CreateEmbed, CreateEmbedAuthor, CreateMessage, EditWebhookMessage, ExecuteWebhook,
    GuildId, MessageReference, MessageReferenceKind, Webhook,
};
use serenity::{
    all::{Context, EventHandler, Reaction, Ready},
    async_trait,
    futures::lock::Mutex,
};

pub struct Handler {
    pub db: Mutex<DatabaseConnection>,
    pub guild_id: GuildId,
    pub board_channel_id: ChannelId,
    pub webhook: Webhook,
    pub min_count: u64,
}

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let Err(e) = self.reaction_add_inner(ctx, reaction).await {
            error!("error on reaction add processing: {e:?}");
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Bot is ready as {}", ready.user.tag());
    }
}

impl Handler {
    async fn reaction_add_inner(&self, ctx: Context, reaction: Reaction) -> anyhow::Result<()> {
        debug!("add reaction {} to {}", reaction.emoji, reaction.message_id);

        let Some(guild_id) = reaction.guild_id else {
            return Ok(());
        };

        if guild_id != self.guild_id {
            return Ok(());
        }

        if reaction.channel_id == self.board_channel_id {
            return Ok(());
        }

        let msg = reaction.message(&ctx.http).await?;
        let reactions: Vec<_> = msg
            .reactions
            .iter()
            .map(|x| (x.reaction_type.clone(), x.count))
            .collect();

        let lock = self.db.lock().await;

        let existing = koi::Entity::find()
            .filter(koi::Column::ReferenceId.eq(msg.id.to_string()))
            .one(&*lock)
            .await?;

        let should_send = reactions.iter().any(|x| x.1 >= self.min_count);

        if should_send {
            let description = reactions
                .iter()
                .filter_map(|(t, count)| {
                    if *count < self.min_count {
                        return None;
                    }

                    Some(format!("{} {}", t, count))
                })
                .join("  ·  ");

            let embed = CreateEmbed::new()
                .author(
                    CreateEmbedAuthor::new(msg.author.display_name()).icon_url(
                        msg.author
                            .avatar_url()
                            .unwrap_or_else(|| msg.author.default_avatar_url()),
                    ),
                )
                .description(description);

            if let Some(existing) = existing {
                let message_id = existing.counter_id;

                self.webhook
                    .edit_message(
                        &ctx.http,
                        message_id.parse::<u64>().unwrap().into(),
                        EditWebhookMessage::new().embed(embed),
                    )
                    .await?;
            } else {
                let counter = self
                    .webhook
                    .execute(&ctx.http, true, ExecuteWebhook::new().embed(embed))
                    .await?
                    .ok_or_else(|| anyhow!("webhook result is None"))?;

                self.board_channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().reference_message(
                            MessageReference::new(MessageReferenceKind::Forward, msg.channel_id)
                                .guild_id(guild_id)
                                .message_id(msg.id)
                                .fail_if_not_exists(true),
                        ),
                    )
                    .await?;

                let koi = koi::ActiveModel {
                    reference_id: ActiveValue::Set(reaction.message_id.to_string()),
                    counter_id: ActiveValue::Set(counter.id.to_string()),
                    ..Default::default()
                };
                let res: InsertResult<_> = koi::Entity::insert(koi).exec(&*lock).await?;
                debug!("insert result: {res:?}");
            }
        }

        Ok(())
    }
}
