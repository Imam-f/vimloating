use super::*;
#[test]
fn search_highlighting_covers_matches_overlapping_the_visible_columns() {
    let line: Vec<_> = "banana".chars().collect();
    let needle: Vec<_> = "ana".chars().collect();
    let prefix = search_prefix(&needle);

    assert_eq!(
        search_highlights(&line, &needle, &prefix, 2, 5, false),
        vec![true, true, true]
    );
}
