//! Shared fixture loaders, assertion helpers, and structure-aware `roxmltree` XML mutators.

use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use cius_ro::{ValidateOptions, validate_bytes};

pub(crate) fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub(crate) fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/xsd")
}

pub(crate) fn load_fixture_bytes(name: &str) -> Vec<u8> {
    fs::read(fixtures_dir().join(name)).unwrap_or_else(|e| panic!("missing fixture {name}: {e}"))
}

pub(crate) fn load_fixture_str(name: &str) -> String {
    String::from_utf8(load_fixture_bytes(name)).expect("UTF-8 XML fixture")
}

pub(crate) fn assert_has_code(xml_or_pdf: &[u8], expected_code: &str) {
    let report = validate_bytes(xml_or_pdf, &ValidateOptions::default());
    assert!(
        report.violations.iter().any(|v| v.code == expected_code),
        "Expected violation code '{expected_code}', got: {:?}",
        report
            .violations
            .iter()
            .map(|v| (&v.code, &v.rule_sub_id, v.severity, &v.message))
            .collect::<Vec<_>>()
    );
}

/// Structure-aware XML fixture builder backed by [`roxmltree`] byte spans.
///
/// Every mutator locates elements by slash-separated local-name ancestor suffix paths
/// (for example, `"Invoice/ID"` or `"AccountingCustomerParty/Party/PartyTaxScheme"`) and
/// panics immediately if the selector does not match uniquely — preventing formatting
/// changes in XML fixtures from silently turning test mutations into no-ops.
#[derive(Clone, Debug)]
pub(crate) struct XmlFixture {
    xml: String,
}

impl XmlFixture {
    pub(crate) fn load(fixture_name: &str) -> Self {
        let xml = load_fixture_str(fixture_name);
        roxmltree::Document::parse(&xml)
            .unwrap_or_else(|e| panic!("fixture {fixture_name} is not well-formed XML: {e}"));
        Self { xml }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.xml
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.xml.as_bytes()
    }

    /// Replaces the inner text of the unique leaf element matching `path`.
    pub(crate) fn set_text(mut self, path: &str, new_text: &str) -> Self {
        let range = {
            let doc = self.parse();
            let node = find_unique_node(&doc, path);
            leaf_text_range(node, path)
        };
        self.xml.replace_range(range, new_text);
        self
    }

    /// Replaces the inner text of the unique leaf element matching `path` and carrying
    /// `attr_name="attr_val"`.
    pub(crate) fn set_text_where_attr(
        mut self,
        path: &str,
        attr_name: &str,
        attr_val: &str,
        new_text: &str,
    ) -> Self {
        let range = {
            let doc = self.parse();
            let segments: Vec<&str> = path.split('/').collect();
            let matches: Vec<_> = doc
                .descendants()
                .filter(|n| {
                    node_matches_path(*n, &segments) && n.attribute(attr_name) == Some(attr_val)
                })
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "expected 1 element matching `{path}[@{attr_name}='{attr_val}']`, found {}",
                matches.len()
            );
            leaf_text_range(matches[0], path)
        };
        self.xml.replace_range(range, new_text);
        self
    }

    /// Removes the unique element matching `path` (along with its surrounding indentation line
    /// when it occupies dedicated lines).
    pub(crate) fn remove(mut self, path: &str) -> Self {
        let node_range = {
            let doc = self.parse();
            find_unique_node(&doc, path).range()
        };
        let bytes = self.xml.as_bytes();
        let mut start = node_range.start;
        let mut end = node_range.end;

        let mut line_start = start;
        while line_start > 0 && matches!(bytes[line_start - 1], b' ' | b'\t') {
            line_start -= 1;
        }
        if line_start == 0 || bytes[line_start - 1] == b'\n' {
            start = line_start;
            if bytes.get(end) == Some(&b'\n') {
                end += 1;
            }
        }

        self.xml.replace_range(start..end, "");
        self
    }

    /// Replaces the entire element matching `path` with `replacement_xml`.
    pub(crate) fn replace_element(mut self, path: &str, replacement_xml: &str) -> Self {
        let range = {
            let doc = self.parse();
            find_unique_node(&doc, path).range()
        };
        self.xml.replace_range(range, replacement_xml);
        self
    }

    /// Inserts `snippet` immediately before the unique element matching `path`.
    pub(crate) fn insert_before(mut self, path: &str, snippet: &str) -> Self {
        let pos = {
            let doc = self.parse();
            find_unique_node(&doc, path).range().start
        };
        self.xml.insert_str(pos, snippet);
        self
    }

    /// Inserts `snippet` immediately after the unique element matching `path`.
    pub(crate) fn insert_after(mut self, path: &str, snippet: &str) -> Self {
        let pos = {
            let doc = self.parse();
            find_unique_node(&doc, path).range().end
        };
        self.xml.insert_str(pos, snippet);
        self
    }

    /// Appends `child_xml` immediately before the closing tag of the unique non-self-closing
    /// element matching `path`.
    pub(crate) fn append_child(mut self, path: &str, child_xml: &str) -> Self {
        let insert_at = {
            let doc = self.parse();
            let range = find_unique_node(&doc, path).range();
            let raw = &self.xml[range.clone()];
            assert!(
                !raw.ends_with("/>"),
                "cannot append_child to self-closing element `{path}`"
            );
            let close_offset = raw
                .rfind("</")
                .unwrap_or_else(|| panic!("missing closing tag on `{path}`"));
            range.start + close_offset
        };
        self.xml.insert_str(insert_at, child_xml);
        self
    }

    /// Updates all attributes named `attr_name` whose value equals `old_val` to `new_val`,
    /// panicking if none match.
    pub(crate) fn set_attr_all(mut self, attr_name: &str, old_val: &str, new_val: &str) -> Self {
        let ranges: Vec<Range<usize>> = {
            let doc = self.parse();
            doc.descendants()
                .filter(roxmltree::Node::is_element)
                .filter_map(|n| n.attribute_node(attr_name))
                .filter(|a| a.value() == old_val)
                .map(|a| a.range_value())
                .collect()
        };
        assert!(
            !ranges.is_empty(),
            "expected at least 1 attribute `{attr_name}=\"{old_val}\"`"
        );
        for range in ranges.into_iter().rev() {
            self.xml.replace_range(range, new_val);
        }
        self
    }

    /// Replaces an exact substring (such as the `<?xml ...?>` declaration) that must occur
    /// exactly once in the document.
    pub(crate) fn replace_once(mut self, needle: &str, replacement: &str) -> Self {
        let count = self.xml.matches(needle).count();
        assert_eq!(
            count, 1,
            "expected `{needle}` to appear once in XML fixture, found {count}"
        );
        self.xml = self.xml.replacen(needle, replacement, 1);
        self
    }

    fn parse(&self) -> roxmltree::Document<'_> {
        roxmltree::Document::parse(&self.xml)
            .unwrap_or_else(|e| panic!("mutated XML is no longer well-formed: {e}\n{}", self.xml))
    }
}

fn node_matches_path(node: roxmltree::Node<'_, '_>, segments: &[&str]) -> bool {
    if !node.is_element() || segments.is_empty() {
        return false;
    }
    let mut current = Some(node);
    for &seg in segments.iter().rev() {
        let Some(el) = current.filter(roxmltree::Node::is_element) else {
            return false;
        };
        if el.tag_name().name() != seg {
            return false;
        }
        current = el.parent();
    }
    true
}

fn find_unique_node<'a>(doc: &'a roxmltree::Document<'a>, path: &str) -> roxmltree::Node<'a, 'a> {
    let segments: Vec<&str> = path.split('/').collect();
    let matches: Vec<_> = doc
        .descendants()
        .filter(|n| node_matches_path(*n, &segments))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected 1 XML element matching `{path}`, found {}",
        matches.len()
    );
    matches[0]
}

fn leaf_text_range(node: roxmltree::Node<'_, '_>, path: &str) -> Range<usize> {
    assert!(
        !node.children().any(|c| c.is_element()),
        "`set_text` target `{path}` has child elements; target a leaf element instead"
    );
    let first = node
        .first_child()
        .unwrap_or_else(|| panic!("`set_text` target `{path}` has no inner text node"));
    let last = node.last_child().expect("has first child");
    first.range().start..last.range().end
}
