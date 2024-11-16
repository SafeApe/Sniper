use dipsniper::config::getConfig;
use dipsniper::db::{self, DBError};
use dipsniper::models;

use db::DB;
use models::{TradeSettings, User, Wallet};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

#[path = "helper.rs"]
pub mod helper;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Debug)]
struct BotError(String);

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bot error: {}", self.0)
    }
}

impl std::error::Error for BotError {}

impl From<DBError> for BotError {
    fn from(err: DBError) -> Self {
        BotError(err.to_string())
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::new(getConfig().getBotToken());

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(|bot: Bot, q: CallbackQuery| async move {
                    let result: HandlerResult = async {
                        if let Some(ref data) = q.data {
                            if data.starts_with("rename_wallet:") {
                                let wallet_address = data.split(':').nth(1).unwrap();
                                handle_rename_wallet(&bot, &q, wallet_address).await?;
                            } else if data.starts_with("delete_wallet:") {
                                let wallet_address = data.split(':').nth(1).unwrap();
                                handle_delete_wallet(&bot, &q, wallet_address).await?;
                            } else {
                                match data.as_str() {
                                    "wallets_menu" => show_wallets_menu(&bot, &q).await?,
                                    "create_wallet" => handle_create_wallet(&bot, &q).await?,
                                    "show_wallets" => handle_show_wallets(&bot, &q).await?,
                                    "trade_settings" => handle_trade_settings(&bot, &q).await?,
                                    "settings" => {
                                        if let Some(msg) = q.message {
                                            show_settings_menu(bot.clone(), msg.regular_message().unwrap().to_owned()).await?;
                                        }
                                    }
                                    "edit_stoploss" => handle_edit_stoploss(&bot, &q).await?,
                                    "edit_takeprofit" => handle_edit_takeprofit(&bot, &q).await?,
                                    "toggle_trailing" => handle_toggle_trailing(&bot, &q).await?,
                                    "edit_trailing_stoploss" => handle_edit_trailing_stoploss(&bot, &q).await?,
                                    "edit_trailing_takeprofit" => handle_edit_trailing_takeprofit(&bot, &q).await?,
                                    "toggle_multiwallet" => handle_toggle_multiwallet(&bot, &q).await?,
                                    "edit_mev_settings" => handle_edit_mev_settings(&bot, &q).await?,
                                    _ => (),
                                }
                            }
                        }
                        Ok(())
                    }.await;
                    
                    if let Err(err) = result {
                        log::error!("Error in callback handler: {}", err);
                    }
                    Ok(())
                })
        )
        .branch(Update::filter_message().endpoint(handle_message));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Display this help message.")]
    Help,
    #[command(description = "Start the bot.")]
    Start,
    #[command(description = "Show settings menu.")]
    Settings,
}

pub async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> HandlerResult {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Start => {
            let db = DB::new().await?;
            let userid = msg.chat.id.0 as i64;
            
            // Create user if not exists
            if db.get_user(userid).await?.is_none() {
                db.create_user(User::new(userid)).await?;
            }
            
            // Ensure trade settings exist
            ensure_trade_settings(&db, userid).await?;
            
            bot.send_message(msg.chat.id, "Welcome to DipSniper Bot! Use /settings to configure your trading preferences.")
                .await?;
        }
        Command::Settings => {
            show_settings_menu(bot, msg).await?;
        }
    }
    Ok(())
}

async fn show_settings_menu(bot: Bot, msg: Message) -> HandlerResult {
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("Wallets", "wallets_menu")],
        vec![InlineKeyboardButton::callback(
            "Trade Settings",
            "trade_settings",
        )],
    ]);

    bot.send_message(msg.chat.id, "Settings Menu:")
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn show_wallets_menu(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "Create Wallet",
                "create_wallet",
            )],
            vec![InlineKeyboardButton::callback(
                "Show Wallets",
                "show_wallets",
            )],
            vec![InlineKeyboardButton::callback("Back", "settings")],
        ]);

        bot.edit_message_text(msg.chat().id, msg.id(), "Wallet Menu:")
            .reply_markup(keyboard)
            .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_create_wallet(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let userid = q.from.id.0 as i64;
        let db = match DB::new().await {
            Ok(db) => db,
            Err(e) => {
                log::error!("Error creating database connection: {}", e);
                bot.edit_message_text(msg.chat().id, msg.id(), "Error creating wallet.")
                    .await?;
                return Err(Box::new(BotError(e.to_string())));
            }
        };

        match db.create_wallet(Wallet::new(userid)).await {
            Ok(_) => {
                bot.edit_message_text(msg.chat().id, msg.id(), "Wallet created.")
                    .await?;
                Ok(())
            }
            Err(e) => {
                log::error!("Error creating wallet: {}", e);
                bot.edit_message_text(msg.chat().id, msg.id(), "Error creating wallet.")
                    .await?;
                Err(Box::new(BotError(e.to_string())))
            }
        }
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_show_wallets(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let userid = q.from.id.0 as i64;
        let db = DB::new().await?;
        let wallets = db.get_wallets(userid).await?;

        let mut text = String::from("Your Wallets:\n\n");
        let mut keyboard = Vec::new();

        if wallets.is_empty() {
            text.push_str("No wallets found.");
        } else {
            for wallet in wallets {
                text.push_str(&format!("🔑 {} ({})\n", wallet.name, wallet.address));
                keyboard.push(vec![
                    InlineKeyboardButton::callback(
                        format!("✏️ Rename {}", wallet.name),
                        format!("rename_wallet:{}", wallet.address),
                    ),
                    InlineKeyboardButton::callback(
                        format!("🗑️ Delete {}", wallet.name),
                        format!("delete_wallet:{}", wallet.address),
                    ),
                ]);
            }
        }

        keyboard.push(vec![InlineKeyboardButton::callback("➕ Create New Wallet", "create_wallet")]);
        keyboard.push(vec![InlineKeyboardButton::callback("🔙 Back", "wallets_menu")]);

        let keyboard = InlineKeyboardMarkup::new(keyboard);

        bot.edit_message_text(msg.chat().id, msg.id(), text)
            .reply_markup(keyboard)
            .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_rename_wallet(bot: &Bot, q: &CallbackQuery, wallet_address: &str) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
            "🔙 Back",
            "show_wallets",
        )]]);

        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            format!("Please send the new name for wallet {}.", wallet_address),
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_delete_wallet(bot: &Bot, q: &CallbackQuery, wallet_address: &str) -> HandlerResult {
    if let Some(msg) = &q.message {
        let db = DB::new().await?;
        db.remove_wallet(wallet_address).await?;

        bot.edit_message_text(msg.chat().id, msg.id(), "Wallet deleted successfully.")
            .reply_markup(InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback("🔙 Back to Wallets", "show_wallets"),
            ]]))
            .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_trade_settings(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let db = DB::new().await?;
        let userid = msg.chat().id.0 as i64;
        let settings = ensure_trade_settings(&db, userid).await?;

        let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
        
        // Back button
        keyboard.push(vec![InlineKeyboardButton::callback(
            "⬅️ Back to Settings",
            "settings",
        )]);

        // Add buttons for each trade setting
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                format!("Stop Loss ({}%)", settings.stoploss),
                "edit_stoploss"
            ),
            InlineKeyboardButton::callback(
                format!("Take Profit ({}%)", settings.takeprofit),
                "edit_takeprofit"
            ),
        ]);
        
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                format!("Trailing {}", if settings.trailing { "✅" } else { "❌" }),
                "toggle_trailing"
            ),
        ]);
        
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                format!("Trail SL ({}%)", settings.trailing_stop_loss),
                "edit_trailing_stoploss"
            ),
            InlineKeyboardButton::callback(
                format!("Trail TP ({}%)", settings.trailing_take_profit),
                "edit_trailing_takeprofit"
            ),
        ]);
        
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                format!("Multi-wallet {}", if settings.multiwallet { "✅" } else { "❌" }),
                "toggle_multiwallet"
            ),
            InlineKeyboardButton::callback("MEV Settings", "edit_mev_settings"),
        ]);

        let keyboard = InlineKeyboardMarkup::new(keyboard);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            format!(
                "🛠 Trade Settings\n\n\
                Stop Loss: {}%\n\
                Take Profit: {}%\n\
                Trailing: {}\n\
                Trailing Stop Loss: {}%\n\
                Trailing Take Profit: {}%\n\
                Multi-wallet: {}\n\
                MEV Enabled Chains: {}\n\n\
                Select a setting to modify:",
                settings.stoploss,
                settings.takeprofit,
                if settings.trailing { "✅" } else { "❌" },
                settings.trailing_stop_loss,
                settings.trailing_take_profit,
                if settings.multiwallet { "✅" } else { "❌" },
                if settings.mev_enabled_chains.is_empty() {
                    "None".to_string()
                } else {
                    settings.mev_enabled_chains.iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ),
        )
        .reply_markup(keyboard)
        .await?;
    }
    Ok(())
}

async fn ensure_trade_settings(db: &DB, userid: i64) -> Result<TradeSettings, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(settings) = db.get_trade_settings(userid).await? {
        Ok(settings)
    } else {
        let settings = TradeSettings::new(userid);
        db.create_trade_settings(settings.clone()).await?;
        Ok(settings)
    }
}

async fn handle_toggle_trailing(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let db = DB::new().await?;
        let userid = msg.chat().id.0 as i64;
        
        let mut settings = ensure_trade_settings(&db, userid).await?;
        settings.trailing = !settings.trailing;
        db.update_trade_settings(settings.clone()).await?;

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            format!("Trailing has been {}. Use the back button to return to settings.",
                if settings.trailing { "enabled" } else { "disabled" }
            ),
        )
        .reply_markup(keyboard)
        .await?;
    }
    Ok(())
}

async fn handle_toggle_multiwallet(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let db = DB::new().await?;
        let userid = msg.chat().id.0 as i64;
        
        let mut settings = ensure_trade_settings(&db, userid).await?;
        settings.multiwallet = !settings.multiwallet;
        db.update_trade_settings(settings.clone()).await?;

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            format!("Multi-wallet has been {}. Use the back button to return to settings.",
                if settings.multiwallet { "enabled" } else { "disabled" }
            ),
        )
        .reply_markup(keyboard)
        .await?;
    }
    Ok(())
}

async fn handle_message(bot: Bot, msg: Message) -> HandlerResult {
    let msgcopy = msg.clone();
    if let Some(text) = msg.text() {
        if let Some(reply) = msg.reply_to_message() {
            if let Some(bot_msg) = reply.text() {
                let db = DB::new().await?;
                let userid = msg.chat.id.0 as i64;
                let mut settings = ensure_trade_settings(&db, userid).await?;

                match bot_msg {
                    msg if msg.contains("stop loss percentage") => {
                        if let Ok(value) = text.parse::<f32>() {
                            if value >= 0.0 && value <= 100.0 {
                                settings.stoploss = value;
                                db.update_trade_settings(settings).await?;
                                bot.send_message(
                                    msgcopy.chat.id,
                                    format!("Stop loss has been updated to {}%", value),
                                ).await?;
                            } else {
                                bot.send_message(
                                    msgcopy.chat.id,
                                    "Please enter a valid percentage between 0 and 100",
                                ).await?;
                            }
                        }
                    }
                    msg if msg.contains("take profit percentage") => {
                        if let Ok(value) = text.parse::<f32>() {
                            if value >= 0.0 && value <= 100.0 {
                                settings.takeprofit = value;
                                db.update_trade_settings(settings).await?;
                                bot.send_message(
                                    msgcopy.chat.id,
                                    format!("Take profit has been updated to {}%", value),
                                ).await?;
                            } else {
                                bot.send_message(
                                    msgcopy.chat.id,
                                    "Please enter a valid percentage between 0 and 100",
                                ).await?;
                            }
                        }
                    }
                    msg if msg.contains("trailing stop loss percentage") => {
                        if let Ok(value) = text.parse::<f32>() {
                            if value >= 0.0 && value <= 100.0 {
                                settings.trailing_stop_loss = value;
                                db.update_trade_settings(settings).await?;
                                bot.send_message(
                                    msgcopy.chat.id,
                                    format!("Trailing stop loss has been updated to {}%", value),
                                ).await?;
                            } else {
                                bot.send_message(
                                    msgcopy.chat.id,
                                    "Please enter a valid percentage between 0 and 100",
                                ).await?;
                            }
                        }
                    }
                    msg if msg.contains("trailing take profit percentage") => {
                        if let Ok(value) = text.parse::<f32>() {
                            if value >= 0.0 && value <= 100.0 {
                                settings.trailing_take_profit = value;
                                db.update_trade_settings(settings).await?;
                                bot.send_message(
                                    msgcopy.chat.id,
                                    format!("Trailing take profit has been updated to {}%", value),
                                ).await?;
                            } else {
                                bot.send_message(
                                    msgcopy.chat.id,
                                    "Please enter a valid percentage between 0 and 100",
                                ).await?;
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(())
}

async fn handle_edit_stoploss(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Enter new stop loss percentage (0-100):\n\nReply to this message with a number.",
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_edit_takeprofit(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Enter new take profit percentage (0-100):\n\nReply to this message with a number.",
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_edit_trailing_stoploss(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Enter new trailing stop loss percentage:\n\nReply to this message with a number.",
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_edit_trailing_takeprofit(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Enter new trailing take profit percentage:\n\nReply to this message with a number.",
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}

async fn handle_edit_mev_settings(bot: &Bot, q: &CallbackQuery) -> HandlerResult {
    if let Some(msg) = &q.message {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback("⬅️ Back", "trade_settings")],
        ]);
        
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "MEV Settings:\nUse /set_mev_chains command to specify chain IDs where MEV should be enabled.",
        )
        .reply_markup(keyboard)
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}
