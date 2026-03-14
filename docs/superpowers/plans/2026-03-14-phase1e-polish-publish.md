# Plan 1E: Polish & Publish — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Increase test coverage across all crates, add benchmarks, and publish core crates to crates.io for open source readiness.

**Architecture:** Add unit tests to undertested crates, expand integration tests, add more benchmarks with Criterion, and prepare `amanclaw-traits` + `amanclaw-plugin-sdk` for crates.io publishing.

**Tech Stack:** Rust, Criterion (benchmarks), cargo-llvm-cov (coverage), crates.io

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/crates/amanclaw-mcp/src/*.rs` | MODIFY | Add unit tests to each module |
| `rust/crates/amanclaw-memory/src/lib.rs` | MODIFY | Add unit tests |
| `rust/crates/amanclaw-security/src/lib.rs` | MODIFY | Add unit tests |
| `rust/crates/amanclaw-llm/src/*.rs` | MODIFY | Add unit tests |
| `rust/crates/amanclaw-core/benches/pipeline.rs` | MODIFY | Add more benchmark scenarios |
| `rust/crates/amanclaw-traits/Cargo.toml` | MODIFY | Prepare for crates.io |
| `rust/crates/amanclaw-plugin-sdk/Cargo.toml` | MODIFY | Prepare for crates.io |

---

## Chunk 1: Test Coverage Expansion

### Task 1: Add tests to amanclaw-security

**Files:**
- Modify: `rust/crates/amanclaw-security/src/lib.rs`

- [ ] **Step 1: Identify existing code and add tests**

Read the security crate source. Add tests for:
- Input sanitization (XSS, SQL injection patterns)
- Rate limiting logic
- HMAC verification
- Any injection detection

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-security -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-security/
git commit -m "test(security): add unit tests for input sanitization and injection detection"
```

---

### Task 2: Add tests to amanclaw-memory

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/lib.rs` (or relevant files)

- [ ] **Step 1: Add tests for SQLite operations**

Test:
- User CRUD (create, read, update, delete)
- Conversation history storage and retrieval
- Community model operations
- Edge cases: duplicate users, empty queries

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-memory -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-memory/
git commit -m "test(memory): add unit tests for SQLite operations"
```

---

### Task 3: Add tests to amanclaw-llm

**Files:**
- Modify: `rust/crates/amanclaw-llm/src/client.rs`
- Modify: `rust/crates/amanclaw-llm/src/tools.rs`
- Modify: `rust/crates/amanclaw-llm/src/prompts.rs`

- [ ] **Step 1: Add tests**

Test:
- Tool definition serialization
- Prompt building with system/user messages
- Tool call parsing from LLM responses
- Error handling for malformed responses

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-llm -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-llm/
git commit -m "test(llm): add unit tests for tool definitions and prompt building"
```

---

### Task 4: Add tests to amanclaw-registry

**Files:**
- Modify: `rust/crates/amanclaw-registry/src/local.rs`
- Modify: `rust/crates/amanclaw-registry/src/manifest.rs`

- [ ] **Step 1: Add tests**

Test:
- Install/uninstall lifecycle
- Manifest parsing (valid and invalid TOML)
- Search by name and description
- Dependency resolution
- Version comparison

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-registry -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-registry/
git commit -m "test(registry): add unit tests for skill install/uninstall and manifest parsing"
```

---

## Chunk 2: Benchmarks

### Task 5: Expand pipeline benchmarks

**Files:**
- Modify: `rust/crates/amanclaw-core/benches/pipeline.rs`

- [ ] **Step 1: Add benchmark scenarios**

Add benchmarks for:
- Pipeline with 0 skills registered
- Pipeline with 10 skills registered
- Pipeline with 50 skills registered
- Skill dispatch overhead (calling a no-op skill)
- Message serialization/deserialization

- [ ] **Step 2: Run benchmarks**

Run: `cd rust && cargo bench --package amanclaw-core`
Expected: Benchmark results printed

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/benches/
git commit -m "bench(core): expand pipeline benchmarks with multi-skill scenarios"
```

---

### Task 6: Add MCP benchmarks

**Files:**
- Create: `rust/crates/amanclaw-mcp/benches/protocol.rs`
- Modify: `rust/crates/amanclaw-mcp/Cargo.toml`

- [ ] **Step 1: Add Criterion dev-dependency**

In `Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "protocol"
harness = false
```

- [ ] **Step 2: Create benchmarks**

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use amanclaw_mcp::protocol::*;

fn bench_json_rpc_serialization(c: &mut Criterion) {
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(1)),
        serde_json::json!({"tools": []}),
    );

    c.bench_function("json_rpc_serialize", |b| {
        b.iter(|| serde_json::to_string(&response).unwrap())
    });
}

fn bench_json_rpc_deserialization(c: &mut Criterion) {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test","arguments":{"query":"hello"}}}"#;

    c.bench_function("json_rpc_deserialize", |b| {
        b.iter(|| serde_json::from_str::<JsonRpcRequest>(json).unwrap())
    });
}

criterion_group!(benches, bench_json_rpc_serialization, bench_json_rpc_deserialization);
criterion_main!(benches);
```

- [ ] **Step 3: Run benchmarks**

Run: `cd rust && cargo bench --package amanclaw-mcp`
Expected: Benchmark results

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-mcp/benches/ rust/crates/amanclaw-mcp/Cargo.toml
git commit -m "bench(mcp): add JSON-RPC serialization benchmarks"
```

---

## Chunk 3: Crates.io Publishing

### Task 7: Prepare amanclaw-traits for crates.io

**Files:**
- Modify: `rust/crates/amanclaw-traits/Cargo.toml`

- [ ] **Step 1: Add crates.io metadata**

```toml
[package]
name = "amanclaw-traits"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "Core trait definitions for AmanClaw AI agent skills and plugins"
repository = "https://github.com/AmanClaw/amanclaw"
homepage = "https://github.com/AmanClaw/amanclaw"
keywords = ["ai", "agent", "plugin", "skill", "islamic"]
categories = ["api-bindings", "development-tools"]
readme = "README.md"
```

- [ ] **Step 2: Create README for the crate**

Create `rust/crates/amanclaw-traits/README.md`:

```markdown
# amanclaw-traits

Core trait definitions for the AmanClaw AI agent.

## Usage

```rust
use amanclaw_traits::skill::{Skill, SkillInput, SkillResult, SkillMetadata};

struct MySkill;

#[async_trait::async_trait]
impl Skill for MySkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "my_skill".into(),
            description: "Does something useful".into(),
            timeout_ms: 10000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        SkillResult { success: true, output: "done".into(), error: None }
    }
}
```
```

- [ ] **Step 3: Dry run publish**

Run: `cd rust/crates/amanclaw-traits && cargo publish --dry-run`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-traits/Cargo.toml rust/crates/amanclaw-traits/README.md
git commit -m "chore(traits): prepare amanclaw-traits for crates.io publishing"
```

---

### Task 8: Prepare amanclaw-plugin-sdk for crates.io

**Files:**
- Modify: `rust/crates/amanclaw-plugin-sdk/Cargo.toml`

- [ ] **Step 1: Add crates.io metadata**

```toml
[package]
name = "amanclaw-plugin-sdk"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "SDK for building AmanClaw WASM plugins"
repository = "https://github.com/AmanClaw/amanclaw"
homepage = "https://github.com/AmanClaw/amanclaw"
keywords = ["ai", "agent", "plugin", "wasm", "sdk"]
categories = ["wasm", "development-tools"]
readme = "README.md"
```

- [ ] **Step 2: Create README for the crate**

Create `rust/crates/amanclaw-plugin-sdk/README.md` with usage examples showing the `amanclaw_plugin!` macro.

- [ ] **Step 3: Dry run publish**

Run: `cd rust/crates/amanclaw-plugin-sdk && cargo publish --dry-run`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-plugin-sdk/Cargo.toml rust/crates/amanclaw-plugin-sdk/README.md
git commit -m "chore(plugin-sdk): prepare amanclaw-plugin-sdk for crates.io publishing"
```

---

## Chunk 4: CI Enhancement

### Task 9: Add coverage reporting to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add coverage job**

Add to ci.yml:

```yaml
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: taiki-e/install-action@cargo-llvm-cov
      - name: Build dashboard (required by clippy/test)
        run: cd dashboard && npm ci && npm run build
      - name: Generate coverage
        run: cd rust && cargo llvm-cov --workspace --lcov --output-path lcov.info
      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: rust/lcov.info
          fail_ci_if_error: false
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add code coverage reporting with cargo-llvm-cov"
```

---

### Task 10: Add Docker healthcheck

**Files:**
- Modify: `rust/Dockerfile`

- [ ] **Step 1: Add healthcheck**

Add before CMD:

```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:8443/health || exit 1
```

- [ ] **Step 2: Commit**

```bash
git add rust/Dockerfile
git commit -m "fix(docker): add healthcheck to Dockerfile"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | Tests: amanclaw-security | 3 |
| 2 | Tests: amanclaw-memory | 3 |
| 3 | Tests: amanclaw-llm | 3 |
| 4 | Tests: amanclaw-registry | 3 |
| 5 | Expand pipeline benchmarks | 3 |
| 6 | Add MCP benchmarks | 4 |
| 7 | Prepare amanclaw-traits for crates.io | 4 |
| 8 | Prepare amanclaw-plugin-sdk for crates.io | 4 |
| 9 | CI coverage reporting | 2 |
| 10 | Docker healthcheck | 2 |

**Total: 10 tasks, 31 steps**

After completing this plan:
- All major crates have unit tests
- Benchmarks cover pipeline and MCP protocol
- `amanclaw-traits` and `amanclaw-plugin-sdk` ready for `cargo publish`
- CI reports code coverage
- Docker has proper healthcheck
