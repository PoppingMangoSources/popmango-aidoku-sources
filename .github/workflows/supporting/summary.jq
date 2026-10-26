map(select(.reason == "compiler-message") | .message) as $messages
| "## Clippy results\n\n| Level | Count |\n| --- | ---: |\n"
  + (["error", "warning", "note", "help"] | map(. as $level | "| \($level) | \([$messages[] | select(.level == $level)] | length) |") | join("\n"))
  + "\n\n## Versions\n\n- \($RUSTC_VERSION)\n- \($CARGO_VERSION)\n- \($CLIPPY_VERSION)\n"
