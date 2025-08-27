# Multi-Progress Support Design

## Overview

This document outlines the design for adding multi-progress bar support to Kopi's ProgressIndicator system. The implementation focuses on providing nested progress bars for operations that have clear parent-child relationships, improving user visibility into long-running operations.

## Goals

- Provide better visibility into nested operations (e.g., download progress during installation)
- Maintain simplicity in implementation and usage
- Avoid visual clutter by showing child progress only for significant operations
- Ensure consistent behavior across different terminal environments

## Non-Goals

- Supporting arbitrary nesting depth (only 1 level of parent-child)
- Maintaining backward compatibility
- Adding new command-line options or configuration flags
- Supporting parallel progress bars (only hierarchical parent-child relationships)

## Current State Analysis

### Current Implementation Limitations

1. **Install Command**: Forces `no_progress=true` for downloads to prevent overlapping progress bars
2. **Cache Refresh**: Uses `SilentProgress` for nested operations
3. **Terminal Corruption**: Risk of overlapping output when multiple progress bars are active

### Operations That Would Benefit

1. **Install Command** (High Impact)
   - Parent: Overall installation steps (1-8)
   - Child: Download progress (bytes/transfer rate) when file > 10MB
   - Child: Cache refresh progress when using Foojay API

2. **Cache Refresh Command** (Medium Impact)
   - Parent: Overall refresh steps (1-N)
   - Child: Individual source fetching progress

## Design Decisions

### Display Criteria

**Unified 10MB Threshold**: Use 10MB as the consistent threshold for displaying child progress bars across all operations.

### Per-Operation Rules

#### Download Operations
- **Content-Length >= 10MB**: Display child progress bar with bytes/transfer rate
- **Content-Length < 10MB**: No child progress bar
- **Content-Length unknown**: No child progress bar

#### Cache Operations
- **Foojay API**: Always display child progress (typically 5-10 seconds)
- **HTTP Metadata**: Display child progress if file size >= 10MB
- **Local Files**: Never display child progress (too fast)

### Terminal Environment Handling

- **TTY Terminal**: Full multi-progress support with animations
- **CI Environment**: Use SimpleProgress (already implemented)
- **Non-TTY**: Use SimpleProgress (already implemented)

## Implementation Plan

### Phase 1: Core Infrastructure

#### 1.1 Update ProgressIndicator Trait

The ProgressIndicator trait will be extended with a new required method `create_child()` that returns a boxed ProgressIndicator instance. This method creates a child progress indicator that can be used for nested operations.

#### 1.2 Implementation Strategy

**SilentProgress**: The `create_child()` method returns a new SilentProgress instance, maintaining silent behavior for nested operations.

**SimpleProgress**: The `create_child()` method returns a SilentProgress instance to avoid log spam in CI environments. This prevents duplicate output messages that would clutter the logs.

**IndicatifProgress**: The `create_child()` method creates an actual child progress bar using indicatif's MultiProgress functionality. The child shares the same MultiProgress instance as the parent, allowing proper visual nesting.

### Phase 2: IndicatifProgress MultiProgress Implementation

The IndicatifProgress struct will be enhanced to support MultiProgress operations:

1. **Root Instance**: Owns a MultiProgress instance and manages the parent progress bar
2. **Child Instance**: References the parent's MultiProgress but manages its own progress bar
3. **Bar Management**: Each instance tracks its own progress bar, with children registering their bars with the parent's MultiProgress

The implementation ensures that child progress bars are properly positioned below their parent and cleaned up when complete.

### Phase 3: Install Command Integration

The install command will be updated to utilize child progress bars during specific operations:

**Download Operation**:
- When starting a download, check the Content-Length header
- If the size is 10MB or larger, create a child progress bar showing bytes downloaded and transfer rate
- If the size is smaller or unknown, only update the parent progress message
- The child progress bar updates continuously during the download and completes when finished

**Cache Refresh Operation**:
- When refreshing cache with Foojay API, always create a child progress bar
- Show the number of packages being processed
- Complete the child when cache refresh finishes

### Phase 4: Cache Refresh Integration

The cache refresh command will implement child progress based on the metadata source type:

**Foojay API Source**:
- Always creates a child progress bar due to the operation typically taking 5-10 seconds
- Shows package processing count and updates as packages are fetched
- Completes with a summary of packages fetched

**HTTP Metadata Source**:
- First checks the file size through HEAD request or similar mechanism
- If 10MB or larger, creates a child progress bar showing download progress
- If smaller, processes without a child progress bar

**Local File Source**:
- Never creates a child progress bar as local file operations are typically instantaneous
- Updates only the parent progress message

## Visual Examples

### Install Command with Large Download
```
Installing temurin@21 [Step 3/8: Downloading]
  ├─ Downloading temurin-21.0.5: 124.5MB / 256.3MB [48%] 2.3MB/s
```

### Cache Refresh with Foojay
```
Refreshing metadata cache [Step 2/5: Fetching sources]
  ├─ Fetching Foojay packages: 1543 / 3217 [47%]
```

### Small File (No Child Progress)
```
Installing temurin@21 [Step 3/8: Downloading package (5.2MB)]
```

## Testing Strategy

### Unit Tests
1. Test `create_child()` for each ProgressIndicator implementation
2. Test that SilentProgress children remain silent
3. Test that SimpleProgress children don't produce output

### Integration Tests
1. Test Install command with various file sizes (< 10MB, > 10MB, unknown)
2. Test Cache refresh with different source types
3. Test in CI environment (should use SimpleProgress)

### Manual Testing
1. Visual inspection of multi-progress display in terminal
2. Test terminal resize during multi-progress operation
3. Test Ctrl+C interruption handling

## Migration Path

Since backward compatibility is not required:

1. Update all ProgressIndicator implementations simultaneously
2. Update all usage sites to handle the new trait method
3. No deprecation period needed

## Future Considerations

While not in current scope, the following could be considered later:

1. Configurable threshold (currently hardcoded to 10MB)
2. More than 1 level of nesting
3. Parallel progress bars for concurrent operations
4. Progress persistence for resumable operations

## Success Criteria

1. Users can see download progress during installation for large files
2. No terminal corruption or overlapping output
3. Performance overhead is negligible (< 1% CPU usage for progress updates)
4. CI/non-TTY environments continue to work without animations
5. Code complexity remains manageable