use sha2::{Sha256, Digest};

pub mod file_helper;
pub mod csv_helper;

pub fn clean_string(s: &str) -> String {
    // Remove BOM and clean other characters
    let s = s.trim_start_matches('\u{feff}'); // Remove BOM
    let s = s.replace('\u{200B}', "");        // Remove zero-width space
    let s = s.replace('\u{200C}', "");        // Remove zero-width non-joiner
    let s = s.replace('\u{00A0}', " ");       // Replace non-breaking space with space
    // Remove control characters (0-31)
    let s = s.chars()
        .filter(|&c| c > '\u{001F}' || c == '\t' || c == '\n' || c == '\r')
        .collect::<String>();
    s.trim().to_string()
}


pub fn calculate_hash(content: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())  // Return as a hex string
}