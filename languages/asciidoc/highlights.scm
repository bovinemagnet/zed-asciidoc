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
