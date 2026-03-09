# Recursively sum all lines including embedded code blocks
def total_lines:
  .blanks + .code + .comments + ([.blobs // {} | .[] | total_lines] | add // 0);

# Configuration
{"Rust": 500, "Markdown": 200} as $limits |

# Exception list: add file paths relative to project root (e.g., "src/generated.rs")
[] as $exceptions |

# Find all files exceeding their language-specific limits
[
  to_entries[]
  | select($limits[.key])
  | .key as $lang
  | $limits[.key] as $limit
  | .value.reports[]
  | select((.stats | total_lines) > $limit)
  | select(.name as $n | $exceptions | all(. as $e | $n != ("./"+$e)))
  | "--- \(.name): \(.stats | total_lines) lines (limit: \($limit))"
] |

# Report results
if length > 0 then
  (join("\n") + "\n\nSome files exceed size limits. Consider refactoring.") | halt_error(1)
else
  "All files within size limits"
end
