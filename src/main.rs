mod audio;
mod input;
mod mixer;
mod sample;

use anyhow::Result;
use clap::Parser;
use crossbeam_channel::bounded;

#[derive(Parser)]
#[command(name = "clack", version)]
struct Cli {
    /// Output volume
    #[arg(
        short = 'v',
        long = "volume",
        default_value_t = 50,
        value_parser = clap::value_parser!(u8).range(0..=100),
        value_name = "0-100"
    )]
    volume: u8,

    /// Print latency stats once per second (not implemented yet)
    #[arg(long)]
    #[allow(dead_code)]
    verbose: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("clack: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Squared taper (DESIGN.md D5)
    let gain = (cli.volume as f32 / 100.0).powi(2);

    let keyboards = input::InputHandler::list_keyboards();
    if keyboards.is_empty() {
        eprintln!("clack: no keyboard devices found under /dev/input.");
        eprintln!("  Is a keyboard connected? Try: ls -l /dev/input/by-id/");
        eprintln!("  If it is, you may lack permission. Fix:");
        eprintln!("       sudo usermod -aG input $USER   (then log out and back in)");
        std::process::exit(1);
    }
    let count = keyboards.len();

    // Input threads -> audio callback. Events carry no data: a press is just "()".
    let (tx, rx) = bounded::<()>(64);

    // The stream must stay alive (and on this thread) for sound to play.
    let _stream = audio::start(gain, rx)?;

    input::InputHandler::start_reading(keyboards, tx);

    println!(
        "clack: listening on {count} keyboard(s), volume {}. Press Ctrl+C to stop.",
        cli.volume
    );

    let (stop_tx, stop_rx) = bounded::<()>(1);
    ctrlc::set_handler(move || {
        let _ = stop_tx.try_send(());
    })?;
    let _ = stop_rx.recv();

    Ok(())
}
