mod input;

use clap::Parser;
use ctrlc;

#[derive(Parser)]
#[command(name = "clack")]
struct Cli {
    #[arg(short = 'v', long = "volume", default_value = "50")]
    volume: u8,

    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.volume > 100 {
        eprintln!("clack: --volume must be between 0 and 100");
        std::process::exit(2);
    }

    let _gain = (cli.volume as f32 / 100.0).powi(2);

    let keyboards = input::InputHandler::list_keyboards();
    if keyboards.is_empty() {
        eprintln!("clack: no keyboard devices found under /dev/input.");
        eprintln!("  Is a keyboard connected? Try: ls -l /dev/input/by-id/");
        std::process::exit(1);
    }

    input::InputHandler::start_reading(keyboards);

    if cli.verbose {
        println!(
            "clack: listening on volume {}. Press Ctrl+C to stop.",
            cli.volume
        );
    }

    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // Keep main alive - input threads handle output
    std::thread::sleep(std::time::Duration::from_secs(3600));
}