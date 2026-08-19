; The `=` of a document title is an external token outside the `titleN` wrapper that carries
; the marker for every other level, so it needs capturing in its own right.
(document_title
  (title_h0_marker) @title)

(document_title
  (line) @title)

[
  (title1)
  (title2)
  (title3)
  (title4)
  (title5)
] @title

[
  (line_comment)
  (block_comment)
] @comment

(document_attr
  (attr_name) @property)

[
  (document_attr_marker)
  (element_attr_marker)
] @punctuation.delimiter

(block_title) @attribute
(block_style) @type
(positional_attr) @attribute
(id) @label
(role) @attribute
(option) @attribute

(block_macro
  (block_macro_name) @keyword
  (target)? @link_uri)

(attribute_name) @attribute
(attribute_value) @variable.parameter

[
  (admonition_note)
  (admonition_tip)
  (admonition_important)
  (admonition_caution)
  (admonition_warning)
] @keyword

[
  (list_marker_star)
  (list_marker_hyphen)
  (list_marker_dot)
  (list_marker_digit)
  (list_marker_geek)
  (list_marker_alpha)
  (description_marker)
] @punctuation.list_marker

; Per-cell specifiers such as `h|`, `m|` and `2+|`.
(table_cell_attr) @attribute

; The header row AsciiDoc gives a table whose first line is followed by a blank one.
(table_header_row
  (table_cell
    (table_cell_content) @emphasis.strong))

[
  (table_block_marker)
  (csv_table_block_marker)
  (dsv_table_block_marker)
  (listing_block_start_marker)
  (listing_block_end_marker)
  (literal_block_marker)
  (passthrough_block_marker)
  (sidebar_block_start_marker)
  (sidebar_block_end_marker)
] @punctuation.special

[
  (listing_block_body)
  (ident_block)
] @text.literal
