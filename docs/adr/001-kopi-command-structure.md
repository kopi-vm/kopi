# ADR-001: Kopi Command Structure

## Status
Proposed

## Context
Kopi is a JDK version management tool that integrates with shells and fetches metadata from foojay.io. We need to design a command structure that is intuitive, consistent with existing version managers, and meets the specific needs of JDK management.

## Decision

### Command Structure Analysis

After analyzing popular version managers (volta, nvm, pyenv, asdf, mise), we identified common patterns:

1. **Volta Pattern**: Simple, focused commands with automatic switching
   - `volta install node@version`
   - `volta pin node@version`
   - `volta list`

2. **asdf Pattern**: Plugin-based with consistent verbs
   - `asdf install java version`
   - `asdf global java version`
   - `asdf local java version`
   - `asdf list java`

3. **pyenv/nvm Pattern**: Environment-focused
   - `pyenv install version`
   - `pyenv global version`
   - `pyenv local version`
   - `pyenv versions`

### Proposed Kopi Commands

#### Core Commands

1. **Installation & Setup**
   ```bash
   kopi install <version>              # Install a specific JDK version
   kopi install <distribution>@<version>  # Install specific distribution
   kopi install --list                 # List available JDK versions from foojay.io
   kopi uninstall <version>            # Remove an installed JDK version
   kopi uninstall <distribution>@<version>  # Remove specific distribution
   ```

2. **Version Management**
   ```bash
   kopi use <version>              # Switch to a JDK version in current shell
   kopi shell                      # Launch new shell with JDK environment configured
   kopi global <version>           # Set default JDK version globally
   kopi local <version>            # Set JDK version for current project
   kopi pin <version>              # Pin JDK version in project config
   ```

3. **Information Commands**
   ```bash
   kopi list                       # List installed JDK versions
   kopi list --remote              # List available versions from foojay.io
   kopi current                    # Show current JDK version and details
   kopi which                      # Show path to current java executable
   ```

4. **Project Configuration**
   ```bash
   kopi init                       # Initialize kopi in current project
   kopi env                        # Show JDK environment variables
   ```

5. **Advanced Features**
   ```bash
   kopi default <distribution>     # Set default distribution for installations
   kopi refresh                    # Update metadata cache from foojay.io
   kopi prune                      # Remove unused JDK versions
   kopi doctor                     # Diagnose kopi installation issues
   ```

#### Command Options

- `--arch <arch>`: Specify architecture (auto-detected by default)
- `--type <type>`: JDK type (jdk, jre)
- `--lts`: Filter/install only LTS versions
- `--latest`: Install latest version matching criteria
- `--quiet/-q`: Suppress output
- `--verbose/-v`: Detailed output

### Version Specification Format

```
kopi install 21                  # Latest Java 21 (uses default distribution)
kopi install 21.0.1              # Specific version (uses default distribution)
kopi install temurin@17.0.2      # Specific distribution and version
kopi install corretto@21         # Latest Java 21 from Amazon Corretto
kopi install zulu@11.0.15        # Zulu JDK version 11.0.15
kopi install 21 --lts            # Latest LTS of Java 21
kopi install latest --lts        # Latest LTS version
```

### Default Distribution
The default distribution is used when no distribution is specified in the install command. Users can change it using:
```
kopi default temurin             # Set Eclipse Temurin as default
kopi default corretto            # Set Amazon Corretto as default
```

#### Supported Distributions
- `temurin` - Eclipse Temurin (formerly AdoptOpenJDK)
- `corretto` - Amazon Corretto
- `zulu` - Azul Zulu
- `oracle` - Oracle JDK
- `graalvm` - GraalVM
- `liberica` - BellSoft Liberica
- `sapmachine` - SAP Machine
- `semeru` - IBM Semeru
- `dragonwell` - Alibaba Dragonwell

### Configuration Files

1. **Global Config**: `~/.kopi/config.toml`
   - Stores default distribution preference
   - Global settings and preferences

2. **Project Version Files**:
   
   **`.java-version`** (Compatibility Mode)
   - Simple version numbers only: `21`, `11.0.2`, `21-ea`
   - No distribution specification
   - Maintains compatibility with GitHub Actions and other tools
   
   **`.kopi-version`** (Native Format)
   - Uses `@` separator: `temurin@21`, `corretto@11.0.2+9`
   - Clear separation between distribution and version
   - No version ranges or wildcards supported

Note: Kopi supports only exact version specifications. No Maven-style ranges (`[1.7,1.8)`), npm-style ranges (`^1.2.3`), or wildcards (`21.*`) are supported.

### Shell Integration

Following Volta's approach with shims:
- Add `~/.kopi/bin` to PATH
- Create shims for `java`, `javac`, `jar`, etc.
- Automatic version switching based on project config

The `kopi shell` command provides an alternative approach:
- Launches a new shell subprocess with JDK environment variables properly configured
- Sets `JAVA_HOME`, updates `PATH` to include JDK bin directory
- Useful for isolated environments or when shim approach isn't suitable
- Respects project-specific JDK versions if launched within a project directory

## Rationale

1. **Simple Core Commands**: Following Volta's philosophy of simplicity
2. **Familiar Patterns**: Using `install`, `list`, `use` aligns with user expectations
3. **Project-Aware**: Automatic switching like Volta, with explicit control like asdf
4. **JDK-Specific Features**: Vendor selection and LTS filtering address Java ecosystem needs
5. **Performance Focus**: Shim-based approach for fast switching
6. **Compatibility**: Support `.java-version` for easy migration

## Consequences

### Positive
- Intuitive for users familiar with other version managers
- Clear separation between global, local, and temporary (use) contexts
- Flexible version specification
- Easy shell integration

### Negative
- Need to maintain shims for all JDK executables
- Metadata caching complexity for offline usage
- Potential conflicts with existing Java installations

## Implementation Priority

1. Phase 1: Core commands (`install`, `list`, `use`, `current`)
2. Phase 2: Project support (`local`, `pin`, config files) and `shell` command
3. Phase 3: Advanced features (`default`, `doctor`, `prune`, `migrate`)
4. Phase 4: Shell completions and enhanced integration

## References
- Volta CLI: https://docs.volta.sh/
- asdf: https://asdf-vm.com/
- foojay.io API documentation