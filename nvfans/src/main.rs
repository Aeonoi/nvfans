use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();

    if args.len() == 1 {
        return Err("Requires an command argument.".into());
    }

    let command = args[1].clone();
    match command.as_str() {
        "daemon" => nvfans_daemon::run()?,
        _ => {
            eprintln!("Unrecognized command: {command}");
            eprintln!("Usage: nvfans <command>");
            return Err("Unrecognized command".into());
        }
    }

    Ok(())
}
