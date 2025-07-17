mod lsp;
mod config;
mod project;
mod analysis;
mod util;
pub mod solc;


use std::io::{self, BufRead, BufReader, Read, Write};
use lsp::handler::handle_request;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut buffer = String::new();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] != "--stdio" {
        eprintln!("Expected --stdio as argument");
        std::process::exit(1);
    }

    loop {
        // --- Parse LSP headers ---
        let mut content_length = 0;
        loop {
            buffer.clear();
            if reader.read_line(&mut buffer).unwrap() == 0 {
                return; // EOF
            }
            if buffer == "\r\n" {
                break; // End of headers
            }
            if buffer.to_lowercase().starts_with("content-length:") {
                let parts: Vec<&str> = buffer.split(':').collect();
                content_length = parts[1].trim().parse::<usize>().unwrap_or(0);
            }
        }

        if content_length == 0 {
            eprintln!("Invalid Content-Length");
            continue;
        }

        // --- Read the actual JSON payload ---
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).unwrap();

        let request_str = String::from_utf8_lossy(&content);

        // --- Handle request ---
        if let Some(response) = handle_request(&request_str) {
            let response_bytes = response.as_bytes();
            let header = format!("Content-Length: {}\r\n\r\n", response_bytes.len());
            writer.write_all(header.as_bytes()).unwrap();
            writer.write_all(response_bytes).unwrap();
            writer.flush().unwrap();
        }
    }
}
