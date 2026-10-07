use super::Theme;

#[derive(Default)]
pub struct UserConfig {
    pub theme: Option<Theme>,
    pub two_d_only: Option<bool>,
}

pub fn user_config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| std::path::PathBuf::from(home).join(".vimfloating"))
}

pub fn load_user_config() -> UserConfig {
    let Some(path) = user_config_path() else {
        return UserConfig::default();
    };
    let Ok(contents) = std::fs::read_to_string(path) else {
        return UserConfig::default();
    };
    parse_user_config(&contents)
}

fn parse_user_config(contents: &str) -> UserConfig {
    let mut settings = UserConfig::default();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "theme" => settings.theme = Theme::parse(value),
            "2d_only" => settings.two_d_only = value.trim().parse().ok(),
            _ => {}
        }
    }
    settings
}

#[cfg(test)]
#[path = "../../tests/unit/config/user.rs"]
mod tests;
