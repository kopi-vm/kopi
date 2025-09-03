# Shim Binary Implementation Design

This directory contains the detailed design documentation for Kopi's shim binary system, which enables seamless JDK version switching.

## 🚀 Getting Started

1. **[Overview](./01-overview.md)** - Start here to understand the goals and design principles
2. **[Architecture](./02-architecture.md)** - Learn about the system architecture and components
3. **[Implementation Details](./03-implementation-details.md)** - Dive into the core shim binary implementation

## 📚 Documentation Structure

### Core Design

| Document                                                 | Description                                       |
| -------------------------------------------------------- | ------------------------------------------------- |
| [Overview](./01-overview.md)                             | High-level overview, goals, and design principles |
| [Architecture](./02-architecture.md)                     | System architecture and component interactions    |
| [Implementation Details](./03-implementation-details.md) | Core shim binary implementation logic             |

### Platform-Specific Implementation

| Document                                                 | Description                                    |
| -------------------------------------------------------- | ---------------------------------------------- |
| [Unix Implementation](./04-unix-implementation.md)       | Linux/macOS specific details (execve, signals) |
| [Windows Implementation](./05-windows-implementation.md) | Windows specific details (CreateProcess, .exe) |

### Key Features

| Document                                                                      | Description                                   |
| ----------------------------------------------------------------------------- | --------------------------------------------- |
| [Version Resolution](../../adr/014-configuration-and-version-file-formats.md) | How shims determine which JDK version to use  |
| [Performance Optimizations](./07-performance-optimizations.md)                | Techniques for minimizing overhead            |
| [Error Handling](./08-error-handling.md)                                      | Error handling and automatic JDK installation |
| [Distribution-Specific Tools](./09-distribution-specific-tools.md)            | Handling vendor-specific tools                |

### Operations & Management

| Document                                                            | Description                                 |
| ------------------------------------------------------------------- | ------------------------------------------- |
| [Installation and Management](./10-shim-installation-management.md) | Shim installation strategy and management   |
| [Tool Discovery](./11-tool-discovery.md)                            | Creating and maintaining curated tool lists |
| [Security Considerations](./12-security.md)                         | Security measures and validations           |

## 💡 Reading Paths

### For Implementers

1. [Overview](./01-overview.md) → [Architecture](./02-architecture.md) → [Implementation Details](./03-implementation-details.md)
2. Choose your platform: [Unix](./04-unix-implementation.md) or [Windows](./05-windows-implementation.md)
3. Review [Performance Optimizations](./07-performance-optimizations.md) and [Error Handling](./08-error-handling.md)

### For Operations

1. [Installation and Management](./10-shim-installation-management.md)
2. [Security Considerations](./12-security.md)

### For Contributors

1. [Tool Discovery](./11-tool-discovery.md)
2. Review existing platform implementations

## 📌 Key Concepts

- **Shim**: A lightweight binary that intercepts Java tool invocations and forwards them to the correct JDK version
- **Version Resolution**: The process of determining which JDK version to use based on project configuration
- **Zero-overhead**: The design goal of minimal performance impact on Java tool invocations

## 🔗 Related Documents

- [ADR-013: Binary Switching Approaches](../../../adr/013-binary-switching-approaches.md) - Architecture decision record
- [Plan](../plan.md) - Implementation plan and timeline

## ❓ FAQ

**Q: Where do I start?**  
A: Begin with the [Overview](./01-overview.md) document.

**Q: I need to implement shims for a new platform?**  
A: Read the [Architecture](./02-architecture.md) and [Implementation Details](./03-implementation-details.md) first, then refer to existing platform implementations.

**Q: How do shims affect performance?**  
A: See [Performance Optimizations](./07-performance-optimizations.md) for benchmarks and optimization techniques.
