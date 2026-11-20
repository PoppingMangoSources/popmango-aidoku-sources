---
name: aidoku-archive
description: Move a source or template whose upstream PR merged off main onto the merged-sources branch (or a dead site onto dead-sources). Use when the owner asks to archive, move or remove a merged source, or a PR they name has merged.
model: inherit
---

# Archiving merged sources

This deletes crates from `main`, so start only when the owner asked for it. The procedure, the
archive table and how a later change starts from upstream are in
[docs/operations/archives.md](../../../docs/operations/archives.md); read it first.

Done when `scripts/check-archives.py --base <before-removal>` passes and both branches are pushed.

## Steps: Preserve merged sources

Confirm the PR is merged in canonical upstream. Preserve the complete main snapshot, tests,
icons and shared path dependencies on `merged-sources`, following section 17 of the root guide.
Publish the archive first, then remove the crate, README entry and unused icon from main.
Keep applicable review guidance and shared-template regressions on main. Replace archived
frontmatter `code:` citations with pinned external references so the wiki retains working links.
Run `scripts/check-archives.py --base <before-removal>` before publishing main; CI and the
push hook run it as well. Later maintenance starts from current upstream, restoring only
needed tests from the archive. Archive branches do not publish packages or need release bumps.
