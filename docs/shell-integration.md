# Kopi Shell Integration Guide

This guide explains how to integrate Kopi with your shell for automatic Java environment management.

## Overview

Kopi provides the `env` command to output shell-specific environment variables. By integrating this command into your shell configuration, you can automatically set up your Java development environment based on your current project.

## Quick Start

### Bash/Zsh

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
# Automatic Java environment setup with Kopi
if command -v kopi &> /dev/null; then
    eval "$(kopi env --quiet)"
fi
```

### Fish

Add to your `~/.config/fish/config.fish`:

```fish
# Automatic Java environment setup with Kopi
if command -sq kopi
    kopi env --quiet | source
end
```

### PowerShell

Add to your PowerShell profile (`$PROFILE`):

```powershell
# Automatic Java environment setup with Kopi
if (Get-Command kopi -ErrorAction SilentlyContinue) {
    kopi env --quiet | Invoke-Expression
}
```

## Advanced Integration

### Directory-Based Switching (like direnv)

For automatic Java version switching when entering project directories:

#### Bash/Zsh with Custom Function

```bash
# Add to ~/.bashrc or ~/.zshrc
kopi_auto_env() {
    if [[ -f ".kopi-version" ]] || [[ -f ".java-version" ]]; then
        eval "$(kopi env --quiet)"
    fi
}

# Hook into directory changes
if [[ -n "$ZSH_VERSION" ]]; then
    # Zsh
    autoload -U add-zsh-hook
    add-zsh-hook chpwd kopi_auto_env
    kopi_auto_env  # Run on shell start
else
    # Bash
    PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND$'\n'}kopi_auto_env"
fi
```

#### Fish with Function

```fish
# Add to ~/.config/fish/functions/kopi_auto_env.fish
function kopi_auto_env --on-variable PWD
    if test -f .kopi-version -o -f .java-version
        kopi env --quiet | source
    end
end

# Run on shell start
kopi_auto_env
```

### Integration with direnv

If you're already using direnv, add to your `.envrc`:

```bash
# .envrc
if has kopi; then
    eval "$(kopi env)"
fi
```

### Shell Prompt Integration

Show the current Java version in your prompt:

#### Bash/Zsh

```bash
# Function to get current Java version
kopi_version() {
    if command -v kopi &> /dev/null; then
        kopi current 2>/dev/null | grep -oE '[0-9]+(\.[0-9]+)*' | head -1
    fi
}

# Bash prompt example
PS1='[\u@\h \W$(kopi_version && echo " java:$(kopi_version)")]$ '

# Zsh prompt example
PROMPT='%n@%m %1~ $(kopi_version >/dev/null && echo "java:$(kopi_version) ")%# '
```

#### Fish

```fish
# Add to ~/.config/fish/functions/fish_prompt.fish
function fish_prompt
    set -l java_version (kopi current 2>/dev/null | string match -r '\d+(\.\d+)*' | head -1)
    if test -n "$java_version"
        set_color yellow
        echo -n "java:$java_version "
        set_color normal
    end
    # ... rest of your prompt
end
```

#### PowerShell

```powershell
# Add to your PowerShell profile
function prompt {
    $javaVersion = & kopi current 2>$null | Select-String -Pattern '\d+(\.\d+)*' | ForEach-Object { $_.Matches[0].Value }
    if ($javaVersion) {
        Write-Host "[java:$javaVersion] " -NoNewline -ForegroundColor Yellow
    }
    # ... rest of your prompt
    return "> "
}
```

## Project Configuration

### Using .kopi-version

Create a `.kopi-version` file in your project root:

```bash
echo "temurin@21" > .kopi-version
```

### Using .java-version (Compatible with jenv)

Create a `.java-version` file:

```bash
echo "21" > .java-version
```

### Version Resolution Order

Kopi resolves versions in this order:
1. `KOPI_JAVA_VERSION` environment variable
2. `.kopi-version` file (searches up directory tree)
3. `.java-version` file (searches up directory tree)
4. Global default (`~/.kopi/config.toml`)

## CI/CD Integration

### GitHub Actions

```yaml
- name: Setup Java with Kopi
  run: |
    # Install kopi (example)
    curl -fsSL https://get.kopi.dev | bash
    
    # Set Java environment
    eval "$(kopi env)"
    
    # Verify
    java -version
```

### GitLab CI

```yaml
before_script:
  - curl -fsSL https://get.kopi.dev | bash
  - eval "$(kopi env)"
  - java -version
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any
    stages {
        stage('Setup') {
            steps {
                sh '''
                    eval "$(kopi env)"
                    java -version
                '''
            }
        }
    }
}
```

## Troubleshooting

### Command Not Found

If you get "command not found" errors:

1. Ensure kopi is in your PATH:
   ```bash
   which kopi
   ```

2. Add kopi to PATH if needed:
   ```bash
   export PATH="$HOME/.kopi/bin:$PATH"
   ```

### Version Not Detected

If kopi doesn't detect your project version:

1. Check file permissions:
   ```bash
   ls -la .kopi-version .java-version
   ```

2. Verify version format:
   ```bash
   cat .kopi-version  # Should be like: temurin@21
   ```

3. Check version resolution:
   ```bash
   kopi env  # Shows which version would be used
   ```

### Shell Detection Issues

If shell detection fails:

1. Explicitly specify shell:
   ```bash
   eval "$(kopi env --shell bash)"
   ```

2. Check your shell:
   ```bash
   echo $SHELL
   echo $0
   ```

## Performance Tips

1. **Use --quiet flag**: Suppresses stderr output for faster execution
   ```bash
   eval "$(kopi env --quiet)"
   ```

2. **Cache shell detection**: For shells where detection is slow
   ```bash
   KOPI_SHELL=bash eval "$(kopi env --shell $KOPI_SHELL)"
   ```

3. **Lazy loading**: Only run kopi when needed
   ```bash
   # Only in directories with Java projects
   if [[ -f "pom.xml" ]] || [[ -f "build.gradle" ]]; then
       eval "$(kopi env --quiet)"
   fi
   ```

## Security Considerations

1. **Verify project files**: Be cautious with `.kopi-version` files from untrusted sources
2. **Use --quiet in automation**: Prevents information leakage via stderr
3. **Validate installations**: Kopi verifies JDK integrity during installation

## Examples

### Development Workflow

```bash
# Enter project directory
cd ~/projects/my-java-app

# Kopi automatically detects .kopi-version
cat .kopi-version
# temurin@21

# Environment is set automatically (if shell integration is configured)
echo $JAVA_HOME
# /home/user/.kopi/jdks/temurin-21.0.5/

# Run your Java application
./gradlew run
```

### Multiple Projects

```bash
# Project A uses Java 21
cd ~/projects/project-a
echo $JAVA_HOME  # /home/user/.kopi/jdks/temurin-21.0.5/

# Project B uses Java 17
cd ~/projects/project-b
echo $JAVA_HOME  # /home/user/.kopi/jdks/temurin-17.0.9/
```

### Override for Testing

```bash
# Temporarily use a different version
KOPI_JAVA_VERSION=temurin@11 eval "$(kopi env)"

# Or use shell command
eval "$(kopi shell 11)"
```

## Best Practices

1. **Use version files**: Commit `.kopi-version` to your repository
2. **Be specific**: Use full version specs (e.g., `temurin@21.0.5`)
3. **Test locally**: Verify shell integration before deploying
4. **Document requirements**: Include Java version in your README
5. **Use quiet mode**: In automated scripts and prompts

## Related Commands

- `kopi shell`: Interactive shell with modified PATH
- `kopi install`: Install specific JDK versions
- `kopi list`: Show installed JDKs
- `kopi current`: Display current JDK information
- `kopi global`: Set system-wide default