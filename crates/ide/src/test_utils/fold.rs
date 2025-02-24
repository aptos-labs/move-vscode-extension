use syntax::{TextRange, TextSize};

/// Extracts ranges, marked with `<tag> </tag>` pairs from the `text`
pub fn extract_tags(mut text: &str, tag: &str) -> (Vec<(TextRange, Option<String>)>, String) {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut ranges = Vec::new();
    let mut res = String::new();
    let mut stack = Vec::new();
    loop {
        match text.find('<') {
            None => {
                res.push_str(text);
                break;
            }
            Some(i) => {
                res.push_str(&text[..i]);
                text = &text[i..];
                if text.starts_with(&open) {
                    let close_open = text.find('>').unwrap();
                    let attr = text[open.len()..close_open].trim();
                    let attr = if attr.is_empty() {
                        None
                    } else {
                        Some(attr.to_owned())
                    };
                    text = &text[close_open + '>'.len_utf8()..];
                    let from = TextSize::of(&res);
                    stack.push((from, attr));
                } else if text.starts_with(&close) {
                    text = &text[close.len()..];
                    let (from, attr) = stack.pop().unwrap_or_else(|| panic!("unmatched </{tag}>"));
                    let to = TextSize::of(&res);
                    ranges.push((TextRange::new(from, to), attr));
                } else {
                    res.push('<');
                    text = &text['<'.len_utf8()..];
                }
            }
        }
    }
    assert!(stack.is_empty(), "unmatched <{tag}>");
    ranges.sort_by_key(|r| (r.0.start(), r.0.end()));
    (ranges, res)
}

#[test]
fn test_extract_tags() {
    let (tags, text) = extract_tags(r#"<tag fn>fn <tag>main</tag>() {}</tag>"#, "tag");
    let actual = tags
        .into_iter()
        .map(|(range, attr)| (&text[range], attr))
        .collect::<Vec<_>>();
    assert_eq!(actual, vec![("fn main() {}", Some("fn".into())), ("main", None),]);
}
