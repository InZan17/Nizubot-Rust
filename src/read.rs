use std::fs;

pub const TOKEN_PATH: &str = "./token";

pub fn read_token() -> String {
    let contents = fs::read_to_string(TOKEN_PATH)
        .expect("Cannot read token file.");
    contents
}