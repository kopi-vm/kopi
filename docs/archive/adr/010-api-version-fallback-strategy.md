# ADR-010: API Version Fallback Strategy

## Status

Decided

## Context

Kopi integrates with the foojay.io DiscoAPI to fetch JDK metadata. The API provides multiple versions (v1.0, v2.0, v3.0) with different response formats:

- **v1.0**: Returns responses as direct JSON arrays `[...]`
- **v2.0**: Returns responses wrapped in a result object `{"result": [...]}`
- **v3.0**: Returns responses wrapped in a result object `{"result": [...]}` (same as v2.0)

Initially, Kopi implemented a version fallback mechanism that would try API versions in order (v3.0 → v2.0 → v1.0) if requests failed. This raised questions about the necessity and complexity of supporting multiple API versions.

## Decision Drivers

1. **Code Simplicity**: Minimize complexity and maintenance burden
2. **Reliability**: Ensure stable API interactions
3. **Future Compatibility**: Prepare for potential API version changes
4. **Error Clarity**: Provide clear error messages for debugging
5. **Performance**: Avoid unnecessary retry attempts

## Considered Options

### Option 1: Support All API Versions with Fallback

Implement automatic fallback from v3.0 to v2.0 to v1.0 when requests fail.

**Advantages:**

- Maximum compatibility with API changes
- Resilience against version-specific outages
- Support for older API versions

**Disadvantages:**

- Increased code complexity
- Difficult to distinguish between network errors and version incompatibility
- Triple the latency in worst-case scenarios
- Different parsing logic needed for v1.0 vs v2.0/v3.0

### Option 2: Support Only Latest API Version (v3.0)

Use only API v3.0 without any fallback mechanism.

**Advantages:**

- Simplified codebase
- Clear error messages with API version
- Faster failure detection
- Single parsing logic
- Easier to test and maintain

**Disadvantages:**

- No automatic resilience to API version deprecation
- Requires code changes when migrating to new API versions

### Option 3: Configurable API Version

Allow users to configure which API version to use via environment variable or config file.

**Advantages:**

- User control over API version
- Useful for debugging
- Gradual migration path

**Disadvantages:**

- Additional configuration complexity
- Users may set incorrect versions
- Still requires maintaining multiple parsing logics

## Decision

We will **support only API v3.0** without version fallback.

### Rationale

1. **Single Point of Failure**: The foojay.io API uses the same server infrastructure for all versions. If the server is down, all versions fail simultaneously. Version fallback provides no real resilience benefit.

2. **API Stability**: The foojay.io API v3.0 has been stable and is actively maintained. There's no indication of imminent deprecation.

3. **Code Simplicity**: Supporting a single API version significantly reduces code complexity:
   - Single parsing logic for wrapped responses
   - No version detection or retry logic
   - Clearer error messages

4. **Future Migration**: When API v4.0 is eventually released:
   - We can evaluate the changes and migration path
   - Update the single `API_VERSION` constant
   - Potentially support both v3.0 and v4.0 during a transition period
   - This is a different scenario than supporting legacy versions

5. **Error Clarity**: All error messages now clearly indicate "API v3.0", making debugging straightforward.

### Implementation

```rust
const API_VERSION: &str = "v3.0";

// All API calls use the constant directly
let url = format!("{}/{}/packages", self.base_url, API_VERSION);

// Error messages include version
Err(KopiError::MetadataFetch(format!(
    "Failed to parse API v{} response: {}",
    API_VERSION, e
)))
```

## Consequences

### Positive

- **Simplified Code**: Removed ~40 lines of fallback logic
- **Faster Failures**: No unnecessary retry attempts across versions
- **Clear Errors**: All errors explicitly mention API v3.0
- **Maintainability**: Single code path is easier to test and debug
- **Future-Ready**: Version constant makes future updates trivial

### Negative

- **No Automatic Fallback**: If v3.0 is deprecated, code changes are required
- **Less Resilient**: No protection against version-specific bugs (theoretical)

### Neutral

- **Migration Path**: Future API version changes require explicit updates
- **Testing**: Only need to test against one API version

## Migration Strategy

When foojay.io releases a new API version (e.g., v4.0):

1. **Evaluation Phase**:
   - Test v4.0 endpoints and response formats
   - Identify breaking changes
   - Assess migration timeline

2. **Transition Phase**:
   - Update code to support both v3.0 and v4.0 if needed
   - Use feature flags or gradual rollout
   - Provide clear communication to users

3. **Deprecation Phase**:
   - Remove v3.0 support once v4.0 is stable
   - Update `API_VERSION` constant
   - Simplify back to single version support

## Future Considerations

1. **API Version Discovery**: Could implement an endpoint that reports available API versions
2. **Compatibility Headers**: Use HTTP headers to indicate client capabilities
3. **Version Negotiation**: Implement content negotiation for API versions
4. **Telemetry**: Track API version usage for informed decisions

## References

- [foojay.io DiscoAPI Documentation](https://api.foojay.io/swagger-ui/)
- [ADR-004: Error Handling Strategy](./004-error-handling-strategy.md)
- [ADR-005: Web API Mocking Strategy](./005-web-api-mocking-strategy.md)
