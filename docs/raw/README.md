# raw/ -- source material

**The LLM does not modify files in this directory.** It only reads them.

## What goes here

Only things that exist **neither in the code nor in a `docs/` page**.

- Design meeting notes, incident post-mortems
- Raw user feedback and interviews
- Third-party or vendor specifications
- Policy excerpts, regulatory replies

File name: `YYYY-MM-DD_title.md`

## What does not go here

- **Copies of source code** -- pages point at live paths via `sources: [code: ...]`
- **Copies of other documents**
- `.env`, API keys, passwords, certificates

The moment you copy something in, there are two originals, and when the code changes only
the copy here goes quietly stale. That is the single failure mode this wiki exists to
avoid.

## Current state

Empty. That is normal.
