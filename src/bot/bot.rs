use dipsniper::config::getConfig;
use dipsniper::db::{self, DBError};
use dipsniper::models;

use db::DB;
use models::Wallet;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

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
        );

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
            Ok(())
        }
        Command::Start => {
            bot.send_message(msg.chat.id, "Welcome! Use /settings to configure the bot.")
                .await?;
            Ok(())
        }
        Command::Settings => {
            show_settings_menu(bot, msg).await?;
            Ok(())
        }
    }
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
        let userid = q.from.id.0;
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
        let userid = q.from.id.0;
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
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Trade settings will be shown here.",
        )
        .await?;
        Ok(())
    } else {
        Err(Box::new(BotError("Message not found".into())))
    }
}
