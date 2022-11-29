pub fn checkonline() -> bool {
    reqwest::blocking::get("https://nmcheck.gnome.org/check_network_status.txt").is_ok()
}