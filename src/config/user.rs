use super::Theme;

#[derive(Default)]
pub struct UserConfig {
    pub theme: Option<Theme>,
    pub two_d_only: Option<bool>,
}

pub fn load_user_config() -> UserConfig {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return UserConfig::default();
    };
    let Ok(contents) = std::fs::read_to_string(std::path::PathBuf::from(home).join(".vimfloating"))
    else {
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
mod tests {
    use super::{Theme, parse_user_config};

    #[test]
    fn parses_theme_and_2d_only_setting() {
        let settings = parse_user_config("theme=everforest\n2d_only=true");
        assert_eq!(settings.theme, Some(Theme::Everforest));
        assert_eq!(settings.two_d_only, Some(true));
    }
}
