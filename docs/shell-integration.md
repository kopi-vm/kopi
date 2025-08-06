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
    eval "$(kopi env)"
fi
```

### Fish

Add to your `~/.config/fish/config.fish`:

```fish
# Automatic Java environment setup with Kopi
if command -sq kopi
    kopi env | source
end
```

### PowerShell

Add to your PowerShell profile:

```powershell
# Check your profile location
echo $PROFILE

# Create profile if it doesn't exist
if (!(Test-Path $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force
}

# Edit your profile
notepad $PROFILE
```

Add the following to your profile:

```powershell
# Automatic Java environment setup with Kopi
if (Get-Command kopi -ErrorAction SilentlyContinue) {
    kopi env | Invoke-Expression
}
```

## Advanced Integration

### Directory-Based Switching (like direnv)

For automatic Java version switching when entering project directories:

#### Bash/Zsh with Custom Function

```bash
# Add to ~/.bashrc or ~/.zshrc
kopi_auto_env() {
    eval "$(kopi env)"
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
    kopi env | source
end

# Run on shell start
kopi_auto_env
```

#### PowerShell with Directory Detection

```powershell
# Add to your PowerShell profile ($PROFILE)
# Auto-switch Java version on directory change
$global:KopiLastPwd = $PWD

function Invoke-KopiAutoEnv {
    if ($PWD.Path -ne $global:KopiLastPwd) {
        $global:KopiLastPwd = $PWD.Path
        if (Get-Command kopi -ErrorAction SilentlyContinue) {
            kopi env | Invoke-Expression
        }
    }
}

# Hook into prompt function for directory change detection
$global:OriginalPromptFunction = $function:prompt

function prompt {
    Invoke-KopiAutoEnv
    & $global:OriginalPromptFunction
}

# Run on shell start
Invoke-KopiAutoEnv
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

When using `kopi env` without arguments, it resolves versions in this order:
1. `KOPI_JAVA_VERSION` environment variable
2. `.kopi-version` file (searches up directory tree)
3. `.java-version` file (searches up directory tree)
4. Global default (`~/.kopi/config.toml`)

You can override this by specifying a version directly:
```bash
eval "$(kopi env 21)"  # Always uses Java 21
```

## Performance Tips

### Bash/Zsh

1. **Cache shell detection**: For shells where detection is slow
   ```bash
   eval "$(kopi env --shell bash)"
   ```

2. **Lazy loading**: Only run kopi in Java project directories
   ```bash
   # Check for common Java project files
   if [[ -f "pom.xml" ]] || [[ -f "build.gradle" ]] || [[ -f "build.gradle.kts" ]]; then
       eval "$(kopi env)"
   fi
   ```

3. **Disable export for faster execution**: When not needed in subshells
   ```bash
   eval "$(kopi env --export=false)"
   ```

### PowerShell

1. **Cache shell detection**: Explicitly specify PowerShell
   ```powershell
   kopi env --shell powershell | Invoke-Expression
   ```

2. **Lazy loading**: Only run kopi in Java project directories
   ```powershell
   # Check for common Java project files
   if ((Test-Path "pom.xml") -or (Test-Path "build.gradle") -or (Test-Path "build.gradle.kts")) {
       kopi env | Invoke-Expression
   }
   ```

## Security Considerations

1. **Verify project files**: Be cautious with `.kopi-version` files from untrusted sources
2. **Validate installations**: Kopi verifies JDK integrity during installation
3. **Review environment changes**: Check what environment variables are being set before evaluation

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

#### Bash/Zsh
```bash
# Project A uses Java 21
cd ~/projects/project-a
echo $JAVA_HOME  # /home/user/.kopi/jdks/temurin-21.0.5/

# Project B uses Java 17
cd ~/projects/project-b
echo $JAVA_HOME  # /home/user/.kopi/jdks/temurin-17.0.9/
```

#### PowerShell
```powershell
# Project A uses Java 21
cd ~/projects/project-a
echo $env:JAVA_HOME  # C:\Users\user\.kopi\jdks\temurin-21.0.5

# Project B uses Java 17
cd ~/projects/project-b
echo $env:JAVA_HOME  # C:\Users\user\.kopi\jdks\temurin-17.0.9
```

### Override for Testing

#### Bash/Zsh
```bash
# For interactive work - launch a new shell (recommended)
kopi shell 11
java -version  # Shows Java 11
exit           # Return to original environment

# For scripts/CI - set specific version temporarily
eval "$(kopi env 11)"
./gradlew test  # Runs with Java 11

# You can also specify distribution
eval "$(kopi env temurin@11)"
```

#### PowerShell
```powershell
# For interactive work - launch a new shell (recommended)
kopi shell 11
java -version  # Shows Java 11
exit           # Return to original environment

# For scripts/CI - set specific version temporarily
kopi env 11 | Invoke-Expression
./gradlew test  # Runs with Java 11

# You can also specify distribution
kopi env temurin@11 | Invoke-Expression
```

## Best Practices

1. **Use version files**: Commit `.kopi-version` to your repository
2. **Be specific**: Use full version specs (e.g., `temurin@21.0.5`)
3. **Test locally**: Verify shell integration before deploying
4. **Document requirements**: Include Java version in your README
5. **Use shell override**: Explicitly specify shell when detection is unreliable

## Related Commands

- `kopi shell <version>`: Launch interactive shell with specified JDK
- `kopi install <version>`: Install specific JDK versions
- `kopi list`: Show installed JDKs
- `kopi current`: Display current JDK information
- `kopi global <version>`: Set system-wide default