mod commands;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Error: No command provided.");
        commands::print_help();
        return;
    }

    match args[1].as_str() {
        "-create" | "-c" => {
            if args.len() >= 4 {
                // Check for at least the file location argument

                let mut file_location: String = "default_qr_code.png".to_string();
                if args.len() == 5 {
                    //Path is specified
                    file_location = args[4].clone();
                    if !file_location.ends_with(".png") {
                        file_location.push_str(".png");
                    }
                }

                let message1 = &args[2];
                let message2 = &args[3];

                commands::create_qr_code(&file_location, message1, message2);
            } else {
                println!("Error: Invalid arguments for -create.");
                commands::print_help();
            }
        }
        "-read" | "-r" => {
            if args.len() == 3 {
                let file_location = &args[2];
                commands::read_actual_code_from_qr_code_file(file_location);
                commands::read_secret_message_from_qr_code_file(file_location);
            } else {
                println!("Error: Invalid arguments for -read.");
                commands::print_help();
            }
        }
        "-help" | "-h" => {
            commands::print_help();
        }
        _ => {
            println!("Error: Unknown command.");
            commands::print_help();
        }
    }
}
