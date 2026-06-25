use std::io::Read;

use claudepowerline::{gather_from_json, render};

fn main() {
    let mut raw = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut raw) {
        eprintln!("claudepowerline: failed to read stdin: {e}");
        std::process::exit(1);
    }

    match gather_from_json(&raw) {
        Ok(data) => print!("{}", render(&data)),
        Err(e) => {
            eprintln!("claudepowerline: invalid status-line JSON: {e}");
            std::process::exit(1);
        }
    }
}
