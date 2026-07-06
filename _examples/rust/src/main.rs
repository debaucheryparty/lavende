use lavende::{LavendeEvent, LavendeManager, LoadResult};
use serenity::{
    all::VoiceState,
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use songbird::SerenityInit;
use std::sync::Arc;
use tokio::sync::RwLock;

struct Handler {
    lavende_manager: Arc<RwLock<Option<LavendeManager>>>,
}

impl Handler {
    fn new() -> Self {
        Self {
            lavende_manager: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("✅ Logged in as {}", ready.user.name);

        let client_id = ready.user.id.to_string();

        let send_to_shard = move |_guild_id: String, payload: serde_json::Value| {
            println!("Sending gateway payload: {:?}", payload);
        };

        let lavende_manager = LavendeManager::new(client_id, send_to_shard);

        let mut events = lavende_manager.subscribe_events();
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                match event {
                    LavendeEvent::TrackStart { guild_id, track } => {
                        println!("🎵 [{}] Playing: {}", guild_id, track.info.title);
                    }
                    LavendeEvent::TrackEnd {
                        guild_id,
                        track,
                        reason,
                    } => {
                        println!(
                            "⏹️  [{}] Track ended: {} (reason: {:?})",
                            guild_id, track.info.title, reason
                        );
                    }
                    LavendeEvent::QueueEnd { guild_id } => {
                        println!("📭 [{}] Queue ended", guild_id);
                    }
                    LavendeEvent::Error { guild_id, message } => {
                        eprintln!("❌ [{}] Error: {}", guild_id, message);
                    }
                    _ => {}
                }
            }
        });

        *self.lavende_manager.write().await = Some(lavende_manager);
        println!("🎵 Lavende Manager initialized");
        println!("📝 Commands: !play <query>, !pause, !resume, !stop, !skip, !volume <0-200>");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || !msg.content.starts_with('!') {
            return;
        }

        let Some(guild_id) = msg.guild_id else {
            return;
        };
        let guild_id_str = guild_id.to_string();

        let args: Vec<&str> = msg.content[1..].trim().split_whitespace().collect();
        if args.is_empty() {
            return;
        }

        let command = args[0].to_lowercase();
        let manager_guard = self.lavende_manager.read().await;
        let Some(ref lavende_mgr) = *manager_guard else {
            return;
        };

        match command.as_str() {
            "play" | "p" => {
                if args.len() < 2 {
                    let _ = msg.reply(&ctx, "Usage: !play <query>").await;
                    return;
                }

                let query = args[1..].join(" ");

                let voice_channel_id = ctx
                    .http
                    .get_user_voice_state(guild_id, msg.author.id)
                    .await
                    .ok()
                    .and_then(|vs| vs.channel_id);

                let Some(channel_id) = voice_channel_id else {
                    let _ = msg.reply(&ctx, "❌ You must be in a voice channel! Join a voice channel first.").await;
                    return;
                };

                let songbird = songbird::get(&ctx)
                    .await
                    .expect("Songbird not initialized");

                match songbird.join(guild_id, channel_id).await {
                    Ok(_handler) => {
                        println!("🔊 Joined voice channel: {}", channel_id);
                    }
                    Err(e) => {
                        let _ = msg
                            .reply(&ctx, format!("❌ Failed to join voice channel: {:?}", e))
                            .await;
                        return;
                    }
                }

                let player = lavende_mgr.get_or_create_player(&guild_id_str);

                player
                    .connect(Some(channel_id.to_string()), true, false)
                    .await;

                let _ = msg
                    .reply(&ctx, format!("🔍 Searching: `{}`...", query))
                    .await;

                match player.search(&query).await {
                    Ok(result) => match result {
                        LoadResult::Empty {} => {
                            let _ = msg.channel_id.say(&ctx.http, "❌ No results found").await;
                        }
                        LoadResult::Track(track) => {
                            player.queue.write().await.add(track.clone());
                            let _ = msg
                                .channel_id
                                .say(
                                    &ctx.http,
                                    format!(
                                        "✅ Added: **{}** by {}",
                                        track.info.title, track.info.author
                                    ),
                                )
                                .await;

                            if let Err(e) = player.play().await {
                                let _ = msg
                                    .channel_id
                                    .say(&ctx.http, format!("❌ Playback error: {}", e))
                                    .await;
                            }
                        }
                        LoadResult::Search(tracks) => {
                            if let Some(track) = tracks.first() {
                                player.queue.write().await.add(track.clone());
                                let _ = msg
                                    .channel_id
                                    .say(
                                        &ctx.http,
                                        format!(
                                            "✅ Added: **{}** by {}",
                                            track.info.title, track.info.author
                                        ),
                                    )
                                    .await;

                                if let Err(e) = player.play().await {
                                    let _ = msg
                                        .channel_id
                                        .say(&ctx.http, format!("❌ Playback error: {}", e))
                                        .await;
                                }
                            }
                        }
                        LoadResult::Playlist(playlist) => {
                            player
                                .queue
                                .write()
                                .await
                                .add_multiple(playlist.tracks.clone());
                            let _ = msg
                                .channel_id
                                .say(
                                    &ctx.http,
                                    format!("📃 Added {} tracks from playlist", playlist.tracks.len()),
                                )
                                .await;

                            if let Err(e) = player.play().await {
                                let _ = msg
                                    .channel_id
                                    .say(&ctx.http, format!("❌ Playback error: {}", e))
                                    .await;
                            }
                        }
                        LoadResult::Error(e) => {
                            let _ = msg
                                .channel_id
                                .say(&ctx.http, format!("❌ Error loading track: {:?}", e))
                                .await;
                        }
                    },
                    Err(e) => {
                        let _ = msg
                            .channel_id
                            .say(&ctx.http, format!("❌ Search failed: {}", e))
                            .await;
                    }
                }
            }
            "pause" => {
                if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                    player.pause(true).await;
                    let _ = msg.react(&ctx, '⏸').await;
                } else {
                    let _ = msg.reply(&ctx, "❌ No active player").await;
                }
            }
            "resume" => {
                if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                    player.resume().await;
                    let _ = msg.react(&ctx, '▶').await;
                } else {
                    let _ = msg.reply(&ctx, "❌ No active player").await;
                }
            }
            "stop" => {
                if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                    player.stop().await;
                    let _ = msg.react(&ctx, '⏹').await;

                    let songbird = songbird::get(&ctx)
                        .await
                        .expect("Songbird not initialized");
                    let _ = songbird.leave(guild_id).await;
                } else {
                    let _ = msg.reply(&ctx, "❌ No active player").await;
                }
            }
            "skip" | "s" => {
                if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                    player.skip().await;
                    let _ = msg.react(&ctx, '⏭').await;
                } else {
                    let _ = msg.reply(&ctx, "❌ No active player").await;
                }
            }
            "volume" | "vol" => {
                if args.len() < 2 {
                    let _ = msg.reply(&ctx, "Usage: !volume <0-200>").await;
                    return;
                }
                if let Ok(vol) = args[1].parse::<u32>() {
                    if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                        player.set_volume(vol).await;
                        let _ = msg.reply(&ctx, format!("🔊 Volume: {}%", vol)).await;
                    } else {
                        let _ = msg.reply(&ctx, "❌ No active player").await;
                    }
                }
            }
            "queue" | "q" => {
                if let Some(player) = lavende_mgr.get_player(&guild_id_str) {
                    let queue = player.queue.read().await;
                    if queue.is_empty() && queue.current.is_none() {
                        let _ = msg.reply(&ctx, "📭 Queue is empty").await;
                    } else {
                        let mut response = String::from("**Queue:**\n");
                        if let Some(ref current) = queue.current {
                            response.push_str(&format!(
                                "▶️ Now: **{}** by {}\n\n",
                                current.info.title, current.info.author
                            ));
                        }
                        for (i, track) in queue.tracks.iter().enumerate().take(10) {
                            response.push_str(&format!(
                                "{}. **{}** by {}\n",
                                i + 1,
                                track.info.title,
                                track.info.author
                            ));
                        }
                        if queue.tracks.len() > 10 {
                            response.push_str(&format!("\n...and {} more", queue.tracks.len() - 10));
                        }
                        let _ = msg.reply(&ctx, response).await;
                    }
                } else {
                    let _ = msg.reply(&ctx, "❌ No active player").await;
                }
            }
            _ => {}
        }
    }

    async fn voice_state_update(&self, _ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        let manager_guard = self.lavende_manager.read().await;
        if let Some(ref lavende_mgr) = *manager_guard {
            let packet = serde_json::json!({
                "t": "VOICE_STATE_UPDATE",
                "d": {
                    "guild_id": new.guild_id.map(|id| id.to_string()),
                    "channel_id": new.channel_id.map(|id| id.to_string()),
                    "user_id": new.user_id.to_string(),
                    "session_id": new.session_id,
                    "deaf": new.deaf,
                    "mute": new.mute,
                    "self_deaf": new.self_deaf,
                    "self_mute": new.self_mute,
                }
            });
            lavende_mgr.send_raw_data(&packet).await;
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new())
        .register_songbird()
        .await
        .expect("Error creating client");

    println!("Starting bot...");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
