mod app;
mod ui;

use std::time::Duration;

use clap::Parser;
use crossterm::event::{EventStream, KeyEventKind};
use futures_util::StreamExt;
use tokio::time::interval;

use app::{App, Event};

#[derive(Parser)]
#[command(name = "bitcoin-tui", about = "Terminal UI for Bitcoin Core")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value_t = 8332)]
    port: u16,

    #[arg(long)]
    rpccookiefile: Option<String>,

    #[arg(long)]
    rpcuser: Option<String>,

    #[arg(long)]
    rpcpassword: Option<String>,

    #[arg(long, group = "network")]
    testnet: bool,

    #[arg(long, group = "network")]
    testnet4: bool,

    #[arg(long, group = "network")]
    regtest: bool,

    #[arg(long, group = "network")]
    signet: bool,

    #[arg(long, default_value_t = 5)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _args = Args::parse();

    let mut terminal = ratatui::init();
    let result = run(&mut terminal).await;
    ratatui::restore();
    result
}

async fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();
    let mut reader = EventStream::new();
    let mut tick = interval(Duration::from_secs(1));

    loop {
        terminal.draw(|frame| ui::render(&app, frame))?;

        tokio::select! {
            _ = tick.tick() => {
                app.update(Event::Tick);
            }
            event = reader.next() => {
                if let Some(Ok(crossterm::event::Event::Key(key))) = event
                    && key.kind == KeyEventKind::Press
                {
                    app.update(Event::Key(key));
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
