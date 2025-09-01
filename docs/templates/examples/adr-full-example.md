# ADR-019: Centralized Caching Strategy

## Metadata
- Type: ADR
- Owner: Platform Team Lead
- Reviewers: Backend Team, SRE Team
- Status: Accepted
  <!-- Proposed: Under discussion | Accepted: Approved and to be implemented | Rejected: Considered but not approved | Deprecated: No longer recommended | Superseded: Replaced by another ADR -->
- Date Created: 2024-09-01

## Links
<!-- Internal project artifacts only. The Links section is mandatory for traceability. If a link does not apply, use "N/A – <reason>". -->
- Requirements: [`docs/tasks/caching/requirements.md`](../../tasks/caching/requirements.md)
- Design: [`docs/tasks/caching/design.md`](../../tasks/caching/design.md)
- Plan: [`docs/tasks/caching/plan.md`](../../tasks/caching/plan.md)
- Related ADRs: ADR-002, ADR-006
- Issue: #123
- PR: #456
- Supersedes: N/A – First caching strategy
- Superseded by: N/A – Current version

## Context
<!-- What problem or architecturally significant requirement motivates this decision? Include constraints, assumptions, scope boundaries, and prior art. Keep value-neutral and explicit. -->
- We need to reduce redundant HTTP requests and disk reads across commands.
- Constraints: No external services; must work offline and on Unix/Windows.
- Tension: Memory footprint vs. performance; freshness vs. determinism.
- Prior art: Similar caching approaches in npm, pip, and cargo.

## Success Metrics (optional)
<!-- Define measurable criteria to evaluate if this decision was successful -->
- API call reduction: >80% cache hit rate in typical usage
- Performance: Cache lookups complete in <10ms
- Storage efficiency: Total cache size <100MB for average user
- Review date: 2025-03-01

## Decision
We will introduce a centralized cache layer with namespaced stores for HTTP, filesystem metadata, and computed results, persisting under the standard Kopi data directory. We will define clear TTL policies and invalidation hooks per namespace.

### Decision Drivers (optional)
- Predictable performance improvements for repeated operations
- Clear ownership and observability of cache behavior

### Considered Options (optional)
- Per-feature ad-hoc caches
- Centralized, namespaced cache (chosen)
- No caching

### Option Analysis (optional)
- Ad-hoc — Pros: Simple locally | Cons: Inconsistent, duplicated logic
- Centralized — Pros: Consistent, observable | Cons: Upfront design effort
- No caching — Pros: Simpler | Cons: Slower repeated operations

## Rationale
Centralizing avoids duplicated eviction logic and enables consistent monitoring, while allowing per-namespace TTLs to balance freshness and performance.

## Consequences
### Positive
- Reduces repeated network/disk I/O
- Enables uniform metrics and debug output for cache hits/misses

### Negative
- Requires careful sizing and eviction to avoid bloat
- Adds complexity to bootstrap and testing

### Neutral (if useful)
- Minor latency on first access; benefits accrue over time

## Implementation Notes (optional)
- Provide `Cache` trait and concrete namespaces (`http`, `fs_meta`, `compute`).
- Expose `--no-cache` flag and `KOPI_CACHE_MAX_MB` env var.
- Emit `DEBUG` logs on misses; summarize stats with `--verbose`.

## Examples (optional)
```bash
kopi fetch --verbose  # prints cache stats at exit
```
```rust
// Pseudocode
let cache = Cache::open(namespace::HTTP)?;
if let Some(v) = cache.get(key) { return Ok(v) }
let v = fetch_remote()?;
cache.put(key, &v, Ttl::hours(1))?;
```

## Platform Considerations (required if applicable)
- Store under `%LOCALAPPDATA%/kopi/cache` (Windows) and `~/.local/share/kopi/cache` (Unix).
- Ensure safe concurrent access (advisory locks or atomic rename writes).

## Security & Privacy (required if applicable)
- Avoid caching secrets; redact tokens in keys/values.
- Respect `NO_PROXY`/`HTTPS_PROXY` without persisting credentials.

## Monitoring & Logging (required if applicable)
- Log hit/miss counts at `INFO` with `--verbose`.
- Emit `TRACE` for per-key events when `KOPI_LOG=trace`.

## Open Questions
<!-- Questions that arose during decision-making but don't block the decision -->
- Should we implement cache pre-warming on startup? → Platform Team → Q2 planning
- What's the optimal default TTL for different cache types? → SRE Team → Performance testing

## External References (optional)
<!-- External standards, specifications, articles, or documentation only -->
- [HTTP Caching RFC 7234](https://tools.ietf.org/html/rfc7234) - HTTP caching semantics
- [Caffeine Cache](https://github.com/ben-manes/caffeine) - High-performance caching library design patterns

---

## Template Usage

For detailed instructions on using this template, see [Template Usage Instructions](../README.md#adr-templates-adrmd-and-adr-litemd) in the templates README.

