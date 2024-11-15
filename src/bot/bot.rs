use dipsniper::config::getConfig;

use dipsniper::db;
use dipsniper::models;
use dipsniper::utils;

use db::DB;
use models::Wallet;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    utils::command::BotCommands,
};
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
        .branch(Update::filter_callback_query().endpoint(handle_callback));

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

pub async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Start => {
            bot.send_message(msg.chat.id, "Welcome! Use /settings to configure the bot.")
                .await?
        }
        Command::Settings => show_settings_menu(bot, msg).await?,
    };
    Ok(())
}

async fn show_settings_menu(bot: Bot, msg: Message) -> ResponseResult<Message> {
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("Wallets", "wallets_menu")],
        vec![InlineKeyboardButton::callback(
            "Trade Settings",
            "trade_settings",
        )],
    ]);

    bot.send_message(msg.chat.id, "Settings Menu:")
        .reply_markup(keyboard)
        .await
}

pub async fn handle_callback(bot: Bot, q: CallbackQuery) -> ResponseResult<()> {
    let qcopy = q.clone();
    if let Some(data) = q.data {
        match data.as_str() {
            "wallets_menu" => show_wallets_menu(&bot, &qcopy).await?,
            "create_wallet" => handle_create_wallet(&bot, &qcopy).await?,
            "show_wallets" => handle_show_wallets(&bot, &qcopy).await?,
            "trade_settings" => handle_trade_settings(&bot, &qcopy).await?,
            _ => {}
        }
    }
    Ok(())
}

async fn show_wallets_menu(bot: &Bot, q: &CallbackQuery) -> ResponseResult<()> {
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
    }
    Ok(())
}

async fn handle_create_wallet(bot: &Bot, q: &CallbackQuery) -> ResponseResult<()> {
    if let Some(msg) = &q.message {
        // Implement wallet creation logic here
        // let message = bot
        //     .edit_message_text(msg.chat().id, msg.id(), "Creating new wallet...")
        //     .await?;
        let userid = q.from.id.0;
        let db = match DB::new().await {
            Ok(db) => db,
            Err(e) => {
                log::error!("Error creating database connection: {}", e);
                bot.edit_message_text(msg.chat().id, msg.id(), "Error creating wallet.")
                    .await?;
                return Ok(());
            }
        };

        match db.create_wallet(Wallet::new(userid)).await {
            Ok(_) => {
                bot.edit_message_text(msg.chat().id, msg.id(), "Wallet created.")
                    .await?;
            }
            Err(e) => {
                log::error!("Error creating wallet: {}", e);
                bot.edit_message_text(msg.chat().id, msg.id(), "Error creating wallet.")
                    .await?;
            }
        }
    }
    Ok(())
}

async fn handle_show_wallets(bot: &Bot, q: &CallbackQuery) -> ResponseResult<()> {
    if let Some(msg) = &q.message {
        // Implement wallet listing logic here
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Your wallets will be shown here...",
        )
        .await?;
    }
    Ok(())
}

async fn handle_trade_settings(bot: &Bot, q: &CallbackQuery) -> ResponseResult<()> {
    if let Some(msg) = &q.message {
        // Implement trade settings logic here
        bot.edit_message_text(
            msg.chat().id,
            msg.id(),
            "Trade settings will be shown here...",
        )
        .await?;
    }
    Ok(())
}
