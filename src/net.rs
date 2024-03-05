pub fn download_monsters(theme: &str, _level: usize) {
    // TODO: default server.
    if let Ok(url) = std::env::var("SERVER_URL") {
        let monsters = reqwest::blocking::get(format!("{url}/monsters/{theme}/1"))
            .unwrap()
            .text()
            .unwrap();
        eprintln!("{monsters:?}");
    }
}

