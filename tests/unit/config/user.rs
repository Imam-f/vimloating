use super::{Theme, parse_user_config};

#[test]
fn parses_theme_and_2d_only_setting() {
    let settings = parse_user_config("theme=everforest\n2d_only=true");
    assert_eq!(settings.theme, Some(Theme::Everforest));
    assert_eq!(settings.two_d_only, Some(true));
}
