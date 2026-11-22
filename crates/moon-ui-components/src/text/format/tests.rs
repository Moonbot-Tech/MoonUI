use crate::text::node::{BlockNode, NodeContext};

use super::html::parse;

/// Renders the block tree as nested variant names, with each leaf's markdown, so a test pins both
/// where a block landed and what it holds.
fn shape(node: &BlockNode) -> String {
    let list = |children: &[BlockNode]| children.iter().map(shape).collect::<Vec<_>>().join(", ");
    match node {
        BlockNode::Root { children, .. } => format!("Root[{}]", list(children)),
        BlockNode::Blockquote { children, .. } => format!("Quote[{}]", list(children)),
        BlockNode::List {
            children, ordered, ..
        } => format!("List(ordered={ordered})[{}]", list(children)),
        BlockNode::ListItem { children, .. } => format!("Item[{}]", list(children)),
        BlockNode::Heading { level, .. } => format!("H{level}({:?})", node.to_markdown()),
        BlockNode::Paragraph(_) => format!("P({:?})", node.to_markdown()),
        BlockNode::Table(_) => format!("Table({:?})", node.to_markdown()),
        BlockNode::Break { .. } => "Break".to_string(),
        other => format!("Other({other:?})"),
    }
}

/// Parses `html` and asserts the shape of every top-level block.
fn assert_shape(html: &str, expected: &str) {
    let mut cx = NodeContext::default();
    let doc = parse(html, &mut cx).unwrap();
    let actual = doc.blocks.iter().map(shape).collect::<Vec<_>>().join(" | ");
    assert_eq!(actual, expected, "html: {html}");
}

/// Catches a change to `html.rs:parse_node` that drops or reorders the inline text standing
/// before a heading, or stops grouping that text with the heading: the text view would lose or
/// misplace the sentence in front of a title.
#[test]
fn html_heading_keeps_the_text_before_it_in_front() {
    assert_shape(
        "<div>Intro <h2>Title <b>bold</b></h2>tail</div>",
        r###"Root[Root[P("Intro"), H2("## Title **bold**")], P("tail")]"###,
    );
    assert_shape("<h1>Only</h1>", r##"H1("# Only")"##);
}

/// Catches a change to `html.rs:parse_node` that loses the text before an image or the image's
/// attributes: a rendered HTML note would show the picture without its lead-in sentence.
#[test]
fn html_image_keeps_the_text_before_it_in_front() {
    assert_shape(
        r#"<div>Lead <img src="https://example.com/a.png" alt="A"></div>"#,
        r#"Root[P("Lead"), P("![A](https://example.com/a.png)")]"#,
    );
    assert_shape(
        r#"<span><img src="https://example.com/b.png" alt="B"></span>"#,
        r#"P("![B](https://example.com/b.png)")"#,
    );
}

/// Catches a change to `html.rs:parse_node` that flattens list items: a nested list must stay
/// inside its item, the text after a nested block must merge into that block's paragraph, and an
/// ordered list must stay ordered.
#[test]
fn html_list_items_keep_nested_blocks_and_merge_text() {
    assert_shape(
        "<ol><li>One <b>two</b><p>para</p>more</li><li><ul><li>x</li></ul></li></ol>",
        r#"List(ordered=true)[Item[P("One **two**"), Root[P("paramore")]], Item[List(ordered=false)[Item[P("x")]]]]"#,
    );
}

/// Catches a change to `html.rs:parse_node` that skips the rows under `thead`/`tbody` or drops
/// the text before the table: an HTML table would render empty or lose its caption line.
#[test]
fn html_table_collects_rows_from_sections() {
    assert_shape(
        "<div>Before<table><thead><tr><th>H</th></tr></thead><tbody><tr><td>a</td><td>b</td></tr></tbody></table></div>",
        r#"Root[P("Before"), Table("H\n\n\n\na\n\n | b")]"#,
    );
}

/// Catches a change to `html.rs:parse_node` that mishandles block containers: a blockquote keeps
/// its children, `br` becomes a break, `style`/`script` vanish, and an empty block leaves nothing.
#[test]
fn html_block_containers_breaks_and_ignored_elements() {
    assert_shape(
        "<blockquote><p>quoted</p></blockquote><style>p{}</style><script>x()</script><div></div>",
        r#"Quote[Root[P("quoted")]]"#,
    );
    assert_shape("<div>a</div><br>", r#"Root[Root[P("a")], Break]"#);
}
