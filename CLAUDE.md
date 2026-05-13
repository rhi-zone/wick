# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Architecture

**Wick** is a minimal expression language. Small, ephemeral, perfectly formed—like a droplet condensed from logic. Just wick it.

Functions + numeric values, compiled to multiple backends (WGSL, Cranelift, Lua).

**Crate structure:**
- `wick-core` - Core AST, types, and expression representation
- `wick-cond` - Conditional backend helpers for domain crates
- `wick-scalar` - Standard scalar math functions (sin, cos, etc.)
- `wick-linalg` - Linear algebra types and operations
- `wick-complex` - Complex number operations
- `wick-quaternion` - Quaternion operations

Backends (wgsl, lua, cranelift) are feature flags within each domain crate.

## Publishing

**Published on [crates.io](https://crates.io/crates/wick-core)** as 7 crates: `wick-core`, `wick-cond`, `wick-scalar`, `wick-linalg`, `wick-complex`, `wick-quaternion`, `wick-all`. All at v0.1.0 (early, in active development). The `wick-wasm` crate is `publish = false` (wasm-pack target).

Note: The old `rhizome-dew-*` names (v0.1.0) were published and immediately yanked. The canonical crate names are `wick-*`.

## Context Is The Only Scarce Resource

Every byte that enters the main session stays in the main session for its entire lifetime. File contents, command output, search results, page text — once read, it lingers in cache and shapes every downstream token. There is no "just looking."

**All exploration runs in subagents.** Investigations, audits, deep dives, surveys, "let me check," "let me find" — if the purpose of a tool sequence is to find out something you don't yet know, it runs in a subagent. Renaming the activity does not change what it is. The subagent returns a distilled summary; the raw output stays in the subagent.

The main session holds only the durable artifacts you are producing: the edit, the commit, the doc update.

Inline tool use is reserved for: reading a known file at a known path, edits/writes you're committing to, a single targeted lookup whose result you'll act on immediately.

## Durability

Subagent reports, mid-session realizations, "I'll remember this" — none of these outlast the session. Anything worth keeping goes into CLAUDE.md, code, docs, or a commit. If it isn't written down, it is gone.

**Commit completed work immediately.** After tests pass, commit. After each phase of a multi-phase plan, commit. Uncommitted work is lost work, and accumulated uncommitted phases lose isolation as well.

**Docs change in the same commit as the code.** New pages enter the sidebar in that commit. There is no follow-up.

When adding crates or features, update README.md and docs/ in the same commit.

## Authenticity

When asked to analyze X, read X. Do not synthesize from conversation memory, prior summaries, or what the file probably says. Claims must correspond to evidence produced this session.

**Something unexpected is a signal.** Surprising output, anomalous numbers, a file containing what it shouldn't — stop and find out why. Do not accept the anomaly and proceed.

## Discipline

Corrections from the user are conversation, not material for new rules. A single correction does not warrant a CLAUDE.md edit. Rules are added when a failure mode is observed repeatedly and the rule names the failure it prevents.

Do not announce actions ("I will now…"). Act.

## Behavioral Patterns

From ecosystem-wide session analysis:

- **Question scope early:** Before implementing, ask whether it belongs in this crate/module
- **Check consistency:** Look at how similar things are done elsewhere in the codebase
- **Implement fully:** No silent arbitrary caps, incomplete pagination, or unexposed trait methods
- **Name for purpose:** Avoid names that describe one consumer
- **Verify before stating:** Don't assert API behavior or codebase facts without checking

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test -q
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**Prefer `cargo test -q`** over `cargo test` — quiet mode only prints failures, significantly reducing output noise and context usage.

**When making the same change across multiple crates**, edit all files first, then build once.

**Minimize file churn.** When editing a file, read it once, plan all changes, and apply them in one pass. Avoid read-edit-build-fail-read-fix cycles by thinking through the complete change before starting.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Commit Convention

Use conventional commits: `type(scope): message`

Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`. Scope is optional but recommended for multi-crate repos.

## Hard Constraints

- No `--no-verify`. Fix the issue or fix the hook.
- No path dependencies in `Cargo.toml` — they couple repos and break independent publishing.
- No interactive git (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang.
- No assuming a tool is missing without checking `nix develop`.
- No special cases — design to avoid them.
- No legacy APIs — one API, update all callers.
- No half measures — migrate ALL callers when adding abstraction.
- No tuples as return types — use structs with named fields.
- Do not add to the monolith — split by domain into sub-crates.
