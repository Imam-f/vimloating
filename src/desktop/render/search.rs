pub(super) fn search_prefix(needle: &[char]) -> Vec<usize> {
    let mut prefix = vec![0; needle.len()];
    let mut matched = 0;
    for index in 1..needle.len() {
        while matched > 0 && needle[index] != needle[matched] {
            matched = prefix[matched - 1];
        }
        if needle[index] == needle[matched] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    prefix
}

pub(super) fn search_highlights(
    line: &[char],
    needle: &[char],
    prefix: &[usize],
    visible_start: usize,
    visible_end: usize,
    whole_word: bool,
) -> Vec<bool> {
    let width = visible_end.saturating_sub(visible_start);
    let mut changes = vec![0isize; width + 1];
    if needle.is_empty() {
        return vec![false; width];
    }

    let mut matched = 0;
    let scan_start = visible_start.saturating_sub(needle.len());
    let scan_end = (visible_end + needle.len()).min(line.len());
    for (col, ch) in line
        .iter()
        .enumerate()
        .skip(scan_start)
        .take(scan_end.saturating_sub(scan_start))
    {
        while matched > 0 && *ch != needle[matched] {
            matched = prefix[matched - 1];
        }
        if *ch == needle[matched] {
            matched += 1;
        }
        if matched == needle.len() {
            let start = col + 1 - needle.len();
            let keyword = |ch: char| ch.is_alphanumeric() || ch == '_';
            if whole_word
                && ((start > 0 && keyword(line[start - 1]))
                    || line.get(col + 1).is_some_and(|&ch| keyword(ch)))
            {
                matched = prefix[matched - 1];
                continue;
            }
            let overlap_start = start.max(visible_start);
            let overlap_end = (start + needle.len()).min(visible_end);
            if overlap_start < overlap_end {
                changes[overlap_start - visible_start] += 1;
                changes[overlap_end - visible_start] -= 1;
            }
            matched = prefix[matched - 1];
        }
    }

    let mut active = 0;
    changes
        .into_iter()
        .take(width)
        .map(|change| {
            active += change;
            active > 0
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
}
