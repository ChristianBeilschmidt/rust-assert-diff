use leptos::prelude::*;
use similar::{ChangeTag, TextDiff};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
#[allow(clippy::must_use_candidate, reason = "irrelevant for Leptos component")]
pub fn App() -> impl IntoView {
    let sample_input = r#"left: Object {"$schema": String("https://vega.github.io/schema/vega-lite/v4.json"), "data": Object {"values": Array [Object {"binStart": Number(0.0), "binEnd": Number(2.5), "Frequency": Number(0)}, Object {"binStart": Number(2.5), "binEnd": Number(5.0), "Frequency": Number(0)}, Object {"binStart": Number(5.0),"binEnd": Number(7.5), "Frequency": Number(0)}, Object {"binStart": Number(7.5), "binEnd": Number(10.0), "Frequency": Number(0)}]}, "mark": String("bar"), "encoding": Object {"x": Object {"field": String("binStart"), "bin": Object {"binned": Bool(true), "step": Number(2.5)}, "axis": Object {"title": String("vegetation")}}, "x2": Object {"field": String("binEnd")}, "y": Object {"field": String("Frequency"), "type": String("quantitative")}}} right: Object {"$schema": String("https://vega.github.io/schema/vega-lite/v4.json"), "data": Object {"values": Array [Object {"binStart": Number(0.0), "binEnd": Number(2.5), "Frequency": Number(2)}, Object {"binStart": Number(2.5), "binEnd": Number(5.0), "Frequency": Number(2)}, Object {"binStart": Number(5.0),"binEnd": Number(7.5), "Frequency": Number(2)}, Object {"binStart": Number(7.5), "binEnd": Number(10.0), "Frequency": Number(0)}]}, "mark": String("bar"), "encoding": Object {"x": Object {"field": String("binStart"), "bin": Object {"binned": Bool(true), "step": Number(2.5)}, "axis": Object {"title": String("")}}, "x2": Object {"field": String("binEnd")}, "y": Object {"field": String("Frequency"), "type": String("quantitative")}}}"#;

    let (raw_input, set_raw_input) = signal(sample_input.to_string());

    let parsed_diff = Memo::new(move |_| {
        let input = raw_input.get();
        let (left, right) = extract_left_right(&input);
        let left_formatted = prettify_rust_debug(&left);
        let right_formatted = prettify_rust_debug(&right);

        let diff = TextDiff::from_lines(&left_formatted, &right_formatted);

        let mut left_lines = Vec::new();
        let mut right_lines = Vec::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    left_lines.push((ChangeTag::Equal, change.to_string()));
                    right_lines.push((ChangeTag::Equal, change.to_string()));
                }
                ChangeTag::Delete => {
                    left_lines.push((ChangeTag::Delete, change.to_string()));
                }
                ChangeTag::Insert => {
                    right_lines.push((ChangeTag::Insert, change.to_string()));
                }
            }
        }

        (left_lines, right_lines)
    });
    let left_lines = move || parsed_diff.get().0;
    let right_lines = move || parsed_diff.get().1;

    view! {
        <div class="container">
            <h2>"Rust assert_eq! Visual Diff Tool"</h2>
            <textarea
                prop:value=move || raw_input.get()
                on:input=move |ev| set_raw_input.set(event_target_value(&ev))
                placeholder="Paste assert_eq! panic output here..."
            />

            <div class="diff-grid">
                <DiffPane title="LEFT (Actual)" title_classes="left-header" lines=left_lines />
                <DiffPane title="RIGHT (Expected)" title_classes="right-header" lines=right_lines />
            </div>
        </div>
    }
}

#[component]
#[allow(clippy::must_use_candidate, reason = "irrelevant for Leptos component")]
fn DiffPane(
    title: &'static str,
    title_classes: &'static str,
    #[prop(into)] lines: Signal<Vec<(ChangeTag, String)>>,
) -> impl IntoView {
    view! {
        <div class="diff-pane">
            <div class=format!("pane-header {title_classes}")>{title}</div>
            {move || lines.get().into_iter().enumerate().map(|(idx, (tag, line))| {
                view! {
                    <div class=format!("line {tag_class}", tag_class = tag.class_name())>
                        <span class="line-num">{idx + 1}</span>
                        <span class="line-content">{line}</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Parses the standard panic text into separate `left` and `right` debug strings.
fn extract_left_right(input: &str) -> (String, String) {
    let clean_input = input
        .replace("assertion failed: `(left == right)`", "")
        .replace("thread 'main' panicked at", "");

    if let Some(left_pos) = clean_input.find("left:")
        && let Some(right_pos) = clean_input.find("right:")
        && left_pos < right_pos
    {
        let left_str = &clean_input[left_pos + 5..right_pos];
        let right_str = &clean_input[right_pos + 6..];

        let right_cleaned = right_str
            .split("note:")
            .next()
            .unwrap_or(right_str)
            .split("', ")
            .next()
            .unwrap_or(right_str);

        return (
            left_str.trim().trim_matches('`').trim().to_string(),
            right_cleaned.trim().trim_matches('`').trim().to_string(),
        );
    }
    (input.to_string(), input.to_string())
}

/// Auto-indents standard Rust `Debug` / `serde_json` struct strings for line-by-line diffing.
fn prettify_rust_debug(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    let mut indent = 0;
    let mut in_quote = false;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                out.push(ch);
            }
            '\\' if in_quote => {
                out.push(ch);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            '{' | '[' if !in_quote => {
                out.push(ch);
                indent += 2;
                out.push('\n');
                out.push_str(&" ".repeat(indent));
            }
            '}' | ']' if !in_quote => {
                indent = indent.saturating_sub(2);
                out.push('\n');
                out.push_str(&" ".repeat(indent));
                out.push(ch);
            }
            ',' if !in_quote => {
                out.push(ch);
                out.push('\n');
                out.push_str(&" ".repeat(indent));
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Trait for styling lines in a diff view.
trait LineClass {
    fn class_name(&self) -> &'static str;
}

impl LineClass for ChangeTag {
    fn class_name(&self) -> &'static str {
        match self {
            ChangeTag::Delete => "diff-del",
            ChangeTag::Insert => "diff-add",
            ChangeTag::Equal => "diff-eq",
        }
    }
}
