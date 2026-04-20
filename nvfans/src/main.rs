use std::{env, error::Error};

const DAEMON_COMMAND: [&str; 2] = ["daemon", "client"];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();

    if args.len() == 1 {
        return Err("Requires a command argument.".into());
    }

    let command = args[1].clone();

    match command.as_str() {
        "daemon" => nvfans_daemon::run(),
        "client" => nvfans_client::run(),
        _ => {
            eprintln!("Unrecognized command: {command}");
            eprintln!("Usage: nvfans <command>");
            eprintln!("Commands:");
            for cmd in DAEMON_COMMAND.iter() {
                eprintln!("  - {cmd}");
            }
            return Err("Unrecognized command".into());
        }
    }
}
