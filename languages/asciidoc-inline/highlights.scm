[
  (monospace)
  (passthrough)
] @text.literal

(emphasis) @emphasis.strong
(italic) @emphasis
(highlight) @emphasis

[
  (link_url)
  (email)
] @link_uri

(uri_label) @link_text

(attribute_reference
  (attribute_name) @constant)

(xref
  (id) @link_uri)

(xref
  (reftext) @link_text)

(inline_macro
  (macro_name) @keyword
  (target)? @link_uri
  (attr)? @attribute)

(id_assignment) @label
(role) @attribute
(escaped_sequence) @string.escape

[
  "["
  "]"
  "{"
  "}"
  "<<"
  ">>"
] @punctuation.bracket
