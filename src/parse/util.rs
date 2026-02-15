pub fn checkonline() -> bool {
    reqwest::blocking::get("https://google.com").is_ok()
}
