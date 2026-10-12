pub mod engine;
pub mod input;
pub mod keys;
pub mod profile;
pub mod safety;
pub mod schedule;

pub fn greeting() -> &'static str {
    "LE Turbo"
}

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to start LE Turbo");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_is_not_empty() {
        assert_eq!(greeting(), "LE Turbo");
    }
}
