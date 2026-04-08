#!/bin/bash
# Injects gotcha files into agent context at SessionStart and PostCompact.
# Outputs to stderr so content appears in agent context.

GOTCHAS_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/knowledge-base/gotchas"

if [ -d "$GOTCHAS_DIR" ]; then
  has_content=false
  for f in "$GOTCHAS_DIR"/*.md; do
    [ -f "$f" ] || continue
    # Skip files that only have a header and "None yet."
    content=$(grep -v '^#' "$f" | grep -v '^$' | grep -v 'None yet' | head -1)
    if [ -n "$content" ]; then
      has_content=true
      break
    fi
  done

  if [ "$has_content" = true ]; then
    echo "--- GOTCHAS (known pitfalls from prior work) ---" >&2
    for f in "$GOTCHAS_DIR"/*.md; do
      [ -f "$f" ] || continue
      content=$(grep -v '^#' "$f" | grep -v '^$' | grep -v 'None yet' | head -1)
      if [ -n "$content" ]; then
        cat "$f" >&2
        echo "" >&2
      fi
    done
    echo "--- END GOTCHAS ---" >&2
  fi
fi
