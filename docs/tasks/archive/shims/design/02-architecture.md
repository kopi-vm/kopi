# Architecture

## Component Overview

```
┌─────────────────┐
│   User Shell    │
├─────────────────┤
│      PATH       │ ← ~/.kopi/shims added to PATH
├─────────────────┤
│   java command  │ → Resolves to ~/.kopi/shims/java
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   kopi-shim     │ ← Single Rust binary (Unix) or individual .exe (Windows)
├─────────────────┤
│ 1. Detect tool  │
│ 2. Find version │
│ 3. Resolve path │
└────────┬────────┘
         │
         ▼ exec() on Unix / CreateProcess() on Windows
┌─────────────────┐
│  Actual JDK     │ ← ~/.kopi/jdks/temurin-17.0.2/bin/java
└─────────────────┘
```

## Directory Structure

```
~/.kopi/
├── bin/
│   └── kopi              # Main Kopi CLI binary
├── shims/
│   ├── java              # Unix: symlink → kopi-shim
│   ├── javac             # Windows: java.exe (copy of kopi-shim.exe)
│   ├── jar
│   ├── jshell
│   └── ...               # All JDK bin/ tools
└── jdks/
    ├── temurin-17.0.2/
    ├── corretto-21.0.1/
    └── ...
```

## Next: [Implementation Details](./03-implementation-details.md)