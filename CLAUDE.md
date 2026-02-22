# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Commands

```bash
# Build and validation
cargo build                           # Dev build
cargo build --release                 # Release build (~3.4MB optimized binary)
cargo test                            # Run all tests (1,017 tests)
cargo clippy -- -D warnings           # Lint (must pass before PR)
cargo fmt --check                     # Check formatting

# Run specific test subsets
cargo test telegram --lib             # Unit tests only
cargo test --test memory_comparison   # Integration tests

# Enable pre-push hook (runs fmt, clippy, tests before push)
git config core.hooksPath .githooks

# Quick local validation
./quick_test.sh                       # ~10 second smoke test
./test_telegram_integration.sh        # Full automated test suite (~2 min)

# Docker-based CI (when available)
./dev/ci.sh all                       # Full CI pipeline locally
```

## High-Level Architecture

ZeroClaw is a **trait-based, pluggable AI agent runtime** optimized for minimal size and maximum extensibility. Every major subsystem is defined as a trait, allowing implementations to be swapped via configuration without code changes.

### Core Traits (Extension Points)

| Trait | Location | Purpose | Factory |
|-------|----------|---------|---------|
| `Provider` | `src/providers/traits.rs` | LLM model backends | `src/providers/mod.rs` |
| `Channel` | `src/channels/traits.rs` | Messaging platforms | `src/channels/mod.rs` |
| `Tool` | `src/tools/traits.rs` | Agent capabilities | `src/tools/mod.rs` |
| `Memory` | `src/memory/traits.rs` | Persistence/backends | `src/memory/mod.rs` |
| `RuntimeAdapter` | `src/runtime/traits.rs` | Platform abstraction | `src/runtime/mod.rs` |
| `Observer` | `src/observability/traits.rs` | Metrics/logging | `src/observability/mod.rs` |

### Key Architectural Patterns

**Factory Pattern**: Each subsystem uses a factory function that maps string keys to boxed trait objects:
```rust
// Example: Provider factory in src/providers/mod.rs
pub fn create_provider(name: &str, api_key: Option<&str>) -> Result<Box<dyn Provider>> {
    match name {
        "openai" => Ok(Box::new(openai::OpenAIProvider::new(api_key))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(api_key))),
        // ...
        _ => bail!("Unknown provider: {name}"),
    }
}
```

**To add a new Provider/Channel/Tool**: Implement the trait → add to factory → tests pass.

### Request Flow

```
CLI Command → Agent Loop → Provider (LLM) → Tool Calls → Runtime Execution → Channel Response
                              ↓
                         Memory (recall/store)
                              ↓
                         Security Policy (enforce)
```

## Important File Locations

| Path | Purpose |
|------|---------|
| `src/main.rs` | CLI entrypoint, command routing via `clap` |
| `src/lib.rs` | Module exports, shared command enums |
| `src/agent/loop_.rs` | Main orchestration loop (tool calling, memory integration) |
| `src/config/schema.rs` | Full config structure (treat as public API) |
| `src/gateway/` | Axum HTTP server (webhooks, pairing, `/webhook` endpoint) |
| `src/security/policy.rs` | Autonomy levels, allowlists, rate limits, filesystem scoping |
| `src/security/pairing.rs` | 6-digit pairing code exchange for gateway auth |
| `src/providers/reliable.rs` | Resilient wrapper around providers (retries, warmup) |
| `src/memory/vector.rs` | Custom hybrid search (FTS5 + cosine similarity) |
| `src/identity.rs` | AIEOS/OpenClaw identity system integration |

## Code Conventions (Required)

### Naming
- Types/traits/enums: `PascalCase`
- Modules/files/functions/variables: `snake_case`
- Constants/statics: `SCREAMING_SNAKE_CASE`
- Trait implementers: Use suffixes (`*Provider`, `*Channel`, `*Tool`, `*Memory`, `*Observer`)
- Factory keys: lowercase, stable (`"openai"`, `"discord"`, `"shell"`)
- Tests: behavior-oriented (`allowlist_denies_unknown_user`)

### Architecture Boundaries
- **Extend via traits first**: Implement traits + factory registration before cross-cutting refactors
- **Dependency direction**: Concrete integrations → trait/config/util (never other concrete integrations)
- **Single responsibility per module**: `agent` = orchestration, `channels` = transport, `providers` = model I/O
- **No premature abstractions**: Extract shared utilities only after rule-of-three evidence

### Security-Critical Paths
- `src/security/**`, `src/runtime/**`, `src/gateway/**`, `src/tools/**`, `src/channels/**`
- **Never log secrets**: Use `src/security/secrets.rs` for encrypted credential storage
- **Default deny**: Allowlists only, no blocklists for access control
- **Workspace scoping**: File tools respect `workspace_only` and `forbidden_paths`

## Configuration as Contract

`src/config/schema.rs` keys are public API. Changes must include:
- Default values
- Compatibility impact
- Migration steps
- Rollback guidance

Key config sections:
- `[channels_config.*]` — each channel's credentials and allowlists
- `[autonomy]` — `workspace_only`, `allowed_commands`, `max_actions_per_hour`
- `[gateway]` — `require_pairing`, `allow_public_bind`
- `[memory]` — backend selection, embedding provider, vector/keyword weights

## Memory System

Custom full-stack search (no external vector DB):
- **Vector DB**: Embeddings as BLOB in SQLite, cosine similarity
- **Keyword**: FTS5 virtual tables with BM25
- **Hybrid merge**: `src/memory/vector.rs` weighted merge function
- **Chunking**: Line-based markdown with heading preservation
- **Caching**: `embedding_cache` table with LRU eviction

## Identity System

Two formats supported:
- **OpenClaw** (default): Markdown files (`IDENTITY.md`, `SOUL.md`, `USER.md`, `AGENTS.md`)
- **AIEOS v1.1**: JSON payload via `identity.format = "aieos"` + `aieos_path` or `aieos_inline`

## Channel Allowlists (Important)

Empty allowlist = **deny all**. `"*"` = allow all. Otherwise = exact-match allowlist.

- Telegram: `@username` (without `@`) or numeric user ID
- Discord: Discord user ID
- Slack: Member ID (starts with `U`)
- WhatsApp: E.164 format (`+1234567890`)

## Gateway Security

- Binds `127.0.0.1` by default — refuses `0.0.0.0` without `allow_public_bind = true` or active tunnel
- Pairing required by default: `POST /pair` with `X-Pairing-Code` header exchanges for bearer token
- All `/webhook` requests require `Authorization: Bearer <token>` header
- Tunnel support: Cloudflare, Tailscale, ngrok, or custom binary

## Rust-Specific Notes

- **Crypto provider**: `rustls::crypto::ring::default_provider().install_default()` called in `main.rs`
- **Async runtime**: `tokio` with feature-optimized selection
- **Release profile**: Optimized for size (`opt-level = "z"`, `lto = true`, `strip = true`)
- **Feature flags**: `browser-native` (chromedriver), `hotload` (file watching), `security-full` (Landlock)

## Testing Patterns

- **Inline tests**: `#[cfg(test)] mod tests {}` at bottom of each file
- **Behavior-oriented names**: `<subject>_<expected_behavior>`
- **Neutral fixtures**: Use `user_a`, `test_user`, `project_bot` (no personal identity data)
- **Identity-like naming**: If unavoidable, use ZeroClaw-scoped roles (`ZeroClawAgent`, `zeroclaw_user`)

## Risk Tiers

- **Low**: docs, tests, chore, isolated refactors
- **Medium**: providers/channels/memory/tools behavior changes
- **High**: `src/security/**`, `src/runtime/**`, `src/gateway/**`, CI workflows, access-control boundaries

## Reference Documentation

- `CONTRIBUTING.md` — Contribution guide, collaboration tracks
- `AGENTS.md` — Agent engineering protocol, detailed principles
- `docs/pr-workflow.md` — PR workflow policy
- `docs/reviewer-playbook.md` — Reviewer checklist
- `docs/ci-map.md` — CI ownership and triage
- `docs/frictionless-security.md` — Security design philosophy
