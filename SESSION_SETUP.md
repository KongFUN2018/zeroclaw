# Security Implementation Session Setup

## Context

This is a **parallel session** for implementing Plan B - the complete security features for ZeroClaw's ClawHub integration.

## Worktree Information

- **Worktree Path**: `/Users/kongfun/Code/github/zeroclaw-security/`
- **Branch**: `feat/security-implementation`
- **Base Branch**: `main`
- **Current Commit**: `db85331` (docs: add comprehensive security implementation plan)

## Implementation Plan

The plan is located at: `docs/plans/2026-02-21-clawhub-security-implementation.md`

This plan implements the complete security system from the detailed design document (`docs/plans/详细设计方案.md`), including:

1. **Extended SIF Format** - Full USF specification with permissions, logic types, dependencies, interfaces, tests, and signatures
2. **Security Scanning Module** - Five security rules (prompt injection, permission consistency, code execution risk, dependency chain, interface boundary)
3. **Signature Verification** - Ed25519 signature verification
4. **Integration** - Hot loader integration, CLI commands

## Required Sub-Skill

Use **superpowers:executing-plans** to execute the implementation plan.

## Quick Start

In the new session:

```bash
# Navigate to worktree
cd /Users/kongfun/Code/github/zeroclaw-security/

# Verify environment
git status
cargo --version
rustc --version

# Start the implementation
# Invoke: superpowers:executing-plans
# Plan file: docs/plans/2026-02-21-clawhub-security-implementation.md
```

## Implementation Guidelines

### TDD Methodology

For each task in the plan:
1. Write the **failing test** first
2. Run the test to verify it fails
3. Implement the **minimal code** to make it pass
4. Run the test to verify it passes
5. **Commit** with descriptive message

### Commits

Commit after each task completion. Use descriptive commit messages:
```
feat(scope): brief description

- Detail 1
- Detail 2

Refs: docs/plans/2026-02-21-clawhub-security-implementation.md Task N
```

### Review Checkpoints

After completing tasks 1-5, run a checkpoint:
- Run all tests: `cargo test --all`
- Build release: `cargo build --release`
- Fix any issues before proceeding

After completing tasks 6-10, run another checkpoint.

After completing all tasks, final checkpoint:
- All tests pass
- Release build succeeds
- Documentation updated
- Manual testing with real skills

## Key Files to Modify

### Core Implementation
- `src/clawhub/sif.rs` - Extend SIF format with security fields
- `src/clawhub/security/` - New security scanning module
- `src/clawhub/signature.rs` - Signature verification (new)

### Integration
- `src/skills/mod.rs` - Integrate security into skill loading
- `src/clawhub/hotload.rs` - Add security scanning to hot loader
- `src/commands/` - Add CLI commands for security operations

### Tests
- `src/clawhub/tests/` - Unit tests for SIF and security
- `tests/integration/` - End-to-end integration tests
- `tests/bench/` - Performance benchmarks

## Dependencies

Add to `Cargo.toml` as needed:
```toml
[dependencies]
ed25519-dalek = "2"        # For signature verification
regex = "1"                 # For pattern matching (already exists)
thiserror = "1"             # For error types (already exists)
```

## Reference Documents

- **Implementation Plan**: `docs/plans/2026-02-21-clawhub-security-implementation.md`
- **Detailed Design**: `docs/plans/详细设计方案.md` (lines 113-1768 for security specs)
- **Current SIF Format**: `src/clawhub/sif.rs`
- **Existing ClawHub**: `src/clawhub/`

## Success Criteria

✅ All 14 tasks completed
✅ All tests passing (`cargo test --all`)
✅ Release build successful (`cargo build --release`)
✅ Backward compatibility maintained (existing SIF files still load)
✅ Documentation updated
✅ Integration tests pass

## After Implementation

When all tasks are complete:

1. **Run final verification**:
   ```bash
   cargo test --all
   cargo build --release
   ```

2. **Merge back to main**:
   ```bash
   cd /Users/kongfun/Code/github/zeroclaw/  # Return to main repo
   git worktree merge feat/security-implementation
   git worktree remove ../zeroclaw-security
   ```

3. **Push to origin**:
   ```bash
   git push origin main
   ```

## Notes

- **Backward Compatibility**: All new SIF fields are optional (Option<T>)
- **Performance**: Security scanning should be fast (< 10ms for typical skills)
- **Testing**: Comprehensive unit, integration, and performance tests required
- **Documentation**: Document all new security features and best practices

---

**Ready to start? Use superpowers:executing-plans with the plan file!**
