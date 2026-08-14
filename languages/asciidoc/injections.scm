((block_macro
  (target) @injection.content)
  (#set! injection.language "asciidoc_inline"))

((table_cell
  (table_cell_content) @injection.content)
  (#set! injection.language "asciidoc_inline"))

((paragraph) @injection.content
  (#set! injection.include-children)
  (#set! injection.language "asciidoc_inline"))

((line) @injection.content
  (#set! injection.include-children)
  (#set! injection.language "asciidoc_inline"))

((section_block
  (element_attr
    (positional_attr
      (block_style))
    (positional_attr) @injection.language)
  (listing_block
    (listing_block_body) @injection.content)))
