# ADR-007: Default JDK Distribution Selection

## Status

Proposed

## Context

Kopi needs to select a default JDK distribution for users who run commands like `kopi install 21` without specifying a distribution. This default should balance reliability, community support, licensing considerations, and user expectations. We conducted research on similar tools and industry practices to make an informed decision.

## Decision

We will use **Eclipse Temurin** as the default JDK distribution for Kopi.

### Research Findings

#### Tool Analysis

1. **SDKMAN** - The most popular JDK version manager
   - **Default**: Eclipse Temurin (formerly AdoptOpenJDK)
   - **Command**: `sdk install java` installs latest Temurin (e.g., 21.0.4-tem)
   - **Rationale**: "The de facto standard for OpenJDK distributions"
   - **Impact**: Sets industry precedent for JDK version managers

2. **jenv** - Environment-based JDK manager
   - **Default**: None (manages pre-installed JDKs only)
   - **Approach**: JDK-agnostic, works with any distribution
   - **Insight**: Some tools avoid choosing defaults entirely

3. **jabba** - Cross-platform JDK manager
   - **Default**: None (requires explicit vendor specification)
   - **Command**: `jabba install adopt@1.8-0`
   - **Approach**: Multi-vendor support without preferences

4. **jdk-manager** - Linux system tool
   - **Default**: None (uses system package manager)
   - **Approach**: Leverages platform-specific tools

#### Industry Consensus

1. **Package Managers**
   - Homebrew: Features Temurin prominently
   - Chocolatey: Temurin as primary OpenJDK option
   - Linux repos: Often include Temurin packages

2. **Container Images**
   - Official Docker images use Temurin as the OpenJDK option
   - Kubernetes operators default to Temurin for Java workloads

3. **Community Recommendations**
   - whichjdk.com: Primary recommendation is Temurin
   - Stack Overflow: Frequent mentions of Temurin for production use
   - Java User Groups: Widespread endorsement

### Distribution Comparison

#### Eclipse Temurin

- **Pros**:
  - Backed by Eclipse Foundation with enterprise sponsors (Red Hat, IBM, Microsoft, Azul)
  - TCK certified for Java compatibility
  - LTS releases with predictable support cycles
  - No licensing fees or restrictions
  - Successor to the popular AdoptOpenJDK
  - Wide platform and architecture support
- **Cons**:
  - May not include proprietary optimizations found in vendor-specific builds

#### Oracle OpenJDK

- **Pros**:
  - Reference implementation
  - Direct from Java's creators
  - Cutting-edge features first
- **Cons**:
  - 6-month support cycle for non-LTS releases
  - No long-term support for free builds
  - Potential confusion with Oracle JDK licensing

### Decision Rationale

1. **Community Standard**: SDKMAN's adoption establishes precedent
2. **Vendor Neutrality**: Avoids favoring any commercial vendor
3. **Enterprise Ready**: TCK certified with proper support cycles
4. **Legal Safety**: Clear, permissive licensing without restrictions
5. **Migration Path**: Natural successor to AdoptOpenJDK
6. **Ecosystem Support**: Wide tooling and platform integration

## Implementation

### Default Behavior

```bash
# These commands will install Temurin by default
kopi install 21              # Installs temurin@21
kopi install 21.0.1          # Installs temurin@21.0.1
kopi install latest --lts    # Installs latest Temurin LTS
```

### User Override

```bash
# Users can change the default distribution
kopi default corretto        # Set Amazon Corretto as default
kopi default oracle          # Set Oracle OpenJDK as default

# Or specify distribution explicitly
kopi install corretto@21     # Install Corretto regardless of default
```

### Configuration

The default distribution is stored in `~/.kopi/config.toml`:

```toml
[defaults]
distribution = "temurin"     # Can be changed by user
```

### First-Run Experience

On first installation without a config file:

1. Use Temurin as the default
2. Create config file with explicit default
3. Show message: "Using Eclipse Temurin as default JDK distribution (change with 'kopi default <distribution>')"

## Consequences

### Positive

- Aligns with industry best practices and user expectations
- Reduces decision fatigue for new users
- Provides reliable, well-supported default option
- Easy migration from other tools using Temurin
- Clear licensing without legal concerns

### Negative

- May not be optimal for users requiring vendor-specific features
- Some users might prefer Oracle OpenJDK as the "official" distribution
- Need to maintain metadata for Temurin across all supported platforms

### Migration Strategy

For users migrating from other tools:

- AdoptOpenJDK users: Seamless transition (Temurin is the successor)
- SDKMAN users: Same default behavior
- Oracle JDK users: Need to explicitly set `kopi default oracle-open-jdk`

## Alternatives Considered

1. **Oracle OpenJDK**
   - Rejected due to short support cycles and potential licensing confusion
2. **Amazon Corretto**
   - Good option but less vendor-neutral than Temurin
   - Could be perceived as AWS-centric
3. **No Default**
   - Would require users to always specify distribution
   - Poor user experience for common use case
4. **User Choice on First Run**
   - Adds friction to initial experience
   - Most users lack context to make informed choice

## References

- SDKMAN Documentation: https://sdkman.io/jdks
- Eclipse Temurin: https://adoptium.net/
- whichjdk.com recommendations
- Docker Official Images: https://hub.docker.com/_/eclipse-temurin
