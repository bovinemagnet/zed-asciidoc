; Inject a paragraph whole, so inline markup can span its lines.
((paragraph) @injection.content
  (#set! injection.include-children)
  (#set! injection.language "AsciiDoc Inline"))

; Lines outside paragraphs contain inline AsciiDoc too (titles, list items, etc.).
([
  (admonition (line) @injection.content)
  (block_title (line) @injection.content)
  (callout_list_item (line) @injection.content)
  (checked_list_item (line) @injection.content)
  (description_list_item (line) @injection.content)
  (document_attr (line) @injection.content)
  (document_title (line) @injection.content)
  (ordered_list_item (line) @injection.content)
  (quoted_block (line) @injection.content)
  (quoted_md_block (line) @injection.content)
  (title1 (line) @injection.content)
  (title2 (line) @injection.content)
  (title3 (line) @injection.content)
  (title4 (line) @injection.content)
  (title5 (line) @injection.content)
  (unordered_list_item (line) @injection.content)
]
  (#set! injection.include-children)
  (#set! injection.language "AsciiDoc Inline"))

; Inject comment
((line_comment) @injection.content
  (#set! injection.language "comment"))

((block_comment) @injection.content
  (#set! injection.language "comment"))
