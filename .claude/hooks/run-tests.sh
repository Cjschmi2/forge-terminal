#!/bin/bash
# Runs test suite after code edits (async, non-blocking).
# Uses TEST_CMD env var from settings.json.

TEST_CMD="${TEST_CMD:-npm test}"
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"

cd "$PROJECT_DIR" || exit 0

# Run tests, capture output
output=$($TEST_CMD 2>&1)
exit_code=$?

if [ $exit_code -ne 0 ]; then
  echo "--- TESTS FAILED ---" >&2
  echo "$output" | tail -30 >&2
  echo "--- END TEST OUTPUT ---" >&2
else
  echo "Tests passed." >&2
fi
