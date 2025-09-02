# ADR-002: Serialization Format for foojay.io Metadata Storage

## Status
Proposed

## Context

Kopi needs to cache JDK metadata fetched from the foojay.io API locally to support:
- Offline operations when network is unavailable
- Faster response times by avoiding repeated API calls
- Reduced load on the foojay.io service
- Consistent metadata across operations

The cached metadata includes:
- JDK distributions (vendors): temurin, corretto, zulu, oracle, graalvm, etc.
- Version information: major versions, full versions (e.g., 21.0.1), LTS status
- Platform/architecture support: x86_64, aarch64 for Linux, macOS, Windows
- Download URLs and checksums for each JDK variant
- Metadata update timestamps for cache invalidation

## Decision Drivers

1. **Performance Requirements**
   - Fast lookups by distribution, version, and architecture
   - Reasonable memory footprint for large datasets
   - Quick serialization/deserialization

2. **Operational Requirements**
   - Support for concurrent access from multiple kopi processes
   - Atomic updates to prevent corruption
   - Ability to partially update data without full rewrite

3. **Development Requirements**
   - Maintainable and debuggable format
   - Minimal external dependencies
   - Type safety with Rust's serde ecosystem
   - Simple migration path for schema changes

4. **User Experience Requirements**
   - Reasonable disk space usage
   - Fast command response times
   - Reliable offline operation

## Considered Options

### Option 1: Single JSON File

Store all metadata in a single JSON file at `~/.kopi/cache/metadata.json`.

**Structure:**
```json
{
  "version": 1,
  "last_updated": "2024-01-20T10:00:00Z",
  "distributions": {
    "temurin": {
      "display_name": "Eclipse Temurin",
      "versions": {
        "21": {
          "latest": "21.0.1",
          "lts": true,
          "releases": {
            "21.0.1": {
              "architectures": {
                "x86_64-pc-linux-gnu": {
                  "download_url": "https://github.com/adoptium/temurin21-binaries/...",
                  "checksum": "sha256:abc123...",
                  "size": 195000000,
                  "archive_type": "tar.gz"
                }
              }
            }
          }
        }
      }
    }
  }
}
```

**Advantages:**
- Simplest implementation using existing `serde_json`
- Human-readable for debugging
- Single file to manage
- Easy backup and restore
- No additional dependencies
- Direct mapping from foojay.io API responses

**Disadvantages:**
- Entire file must be loaded into memory
- File rewrite required for any update
- File locking complexity for concurrent access
- Performance degrades with size (potentially 10-50MB)
- No built-in query optimization

**Implementation:**
```rust
use serde::{Deserialize, Serialize};
use fs2::FileExt;
use std::fs::File;

#[derive(Serialize, Deserialize)]
struct MetadataCache {
    version: u32,
    last_updated: DateTime<Utc>,
    distributions: HashMap<String, Distribution>,
}

impl MetadataCache {
    fn load() -> Result<Self> {
        let path = cache_path()?;
        let file = File::open(&path)?;
        
        // Advisory lock for concurrent reads
        file.lock_shared()?;
        let cache = serde_json::from_reader(&file)?;
        file.unlock()?;
        
        Ok(cache)
    }
    
    fn save(&self) -> Result<()> {
        let path = cache_path()?;
        let temp_path = path.with_extension("tmp");
        
        // Write to temporary file
        let temp_file = File::create(&temp_path)?;
        temp_file.lock_exclusive()?;
        serde_json::to_writer_pretty(&temp_file, self)?;
        temp_file.sync_all()?;
        temp_file.unlock()?;
        
        // Atomic rename
        std::fs::rename(temp_path, path)?;
        Ok(())
    }
}
```

### Option 2: SQLite Database

Use SQLite embedded database at `~/.kopi/cache/metadata.db`.

**Schema:**
```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY
);

CREATE TABLE distributions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    last_updated TIMESTAMP
);

CREATE TABLE jdk_versions (
    id INTEGER PRIMARY KEY,
    distribution_id INTEGER NOT NULL,
    major_version INTEGER NOT NULL,
    full_version TEXT NOT NULL,
    is_lts BOOLEAN NOT NULL DEFAULT FALSE,
    is_latest BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (distribution_id) REFERENCES distributions(id),
    UNIQUE(distribution_id, full_version)
);

CREATE TABLE jdk_artifacts (
    id INTEGER PRIMARY KEY,
    version_id INTEGER NOT NULL,
    architecture TEXT NOT NULL,
    os TEXT NOT NULL,
    download_url TEXT NOT NULL,
    checksum_type TEXT NOT NULL,
    checksum_value TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    archive_type TEXT NOT NULL,
    FOREIGN KEY (version_id) REFERENCES jdk_versions(id),
    UNIQUE(version_id, architecture, os)
);

-- Indexes for common queries
CREATE INDEX idx_dist_major ON jdk_versions(distribution_id, major_version);
CREATE INDEX idx_version_full ON jdk_versions(full_version);
CREATE INDEX idx_artifact_arch ON jdk_artifacts(architecture, os);
```

**Advantages:**
- ACID transactions for data integrity
- Built-in concurrent access handling
- Efficient queries with indexes
- Partial updates without full rewrites
- Can handle very large datasets
- Rich query capabilities with SQL

**Disadvantages:**
- Additional dependency (`rusqlite` ~5MB)
- Complex schema management and migrations
- Not human-readable
- Harder to debug issues
- Overkill for simple key-value lookups
- More complex error handling

**Implementation:**
```rust
use rusqlite::{Connection, Transaction};

struct MetadataDb {
    conn: Connection,
}

impl MetadataDb {
    fn new() -> Result<Self> {
        let path = db_path()?;
        let conn = Connection::open(path)?;
        
        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        
        // Initialize schema if needed
        Self::migrate(&conn)?;
        
        Ok(Self { conn })
    }
    
    fn find_jdk(
        &self,
        dist: &str,
        version: &str,
        arch: &str,
        os: &str,
    ) -> Result<JdkArtifact> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT a.download_url, a.checksum_type, a.checksum_value, a.size_bytes
             FROM jdk_artifacts a
             JOIN jdk_versions v ON a.version_id = v.id
             JOIN distributions d ON v.distribution_id = d.id
             WHERE d.name = ?1 AND v.full_version = ?2 
                   AND a.architecture = ?3 AND a.os = ?4"
        )?;
        
        stmt.query_row([dist, version, arch, os], |row| {
            Ok(JdkArtifact {
                download_url: row.get(0)?,
                checksum_type: row.get(1)?,
                checksum_value: row.get(2)?,
                size_bytes: row.get(3)?,
            })
        })
    }
    
    fn update_distribution(&mut self, name: &str, data: Distribution) -> Result<()> {
        let tx = self.conn.transaction()?;
        
        // Delete old data
        tx.execute("DELETE FROM distributions WHERE name = ?1", [name])?;
        
        // Insert new data
        Self::insert_distribution(&tx, name, &data)?;
        
        tx.commit()?;
        Ok(())
    }
}
```

### Option 3: Binary Serialization (bincode)

Store metadata in binary format at `~/.kopi/cache/metadata.bin`.

**Advantages:**
- Smallest file size (30-50% smaller than JSON)
- Fastest serialization/deserialization
- Direct struct mapping without parsing
- Existing Rust ecosystem support

**Disadvantages:**
- Not human-readable
- Version compatibility issues
- Debugging difficulties
- Still requires full file load
- Schema evolution complexity
- Corruption harder to recover from

**Implementation:**
```rust
use bincode::{config, Decode, Encode};

#[derive(Encode, Decode)]
struct MetadataCache {
    #[bincode(with_serde)]
    version: u32,
    #[bincode(with_serde)]
    data: CacheData,
}

impl MetadataCache {
    fn load() -> Result<Self> {
        let path = cache_path()?;
        let data = std::fs::read(&path)?;
        
        let config = config::standard();
        let cache = bincode::decode_from_slice(&data, config)?.0;
        
        Ok(cache)
    }
    
    fn save(&self) -> Result<()> {
        let config = config::standard();
        let encoded = bincode::encode_to_vec(self, config)?;
        
        // Atomic write
        let path = cache_path()?;
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, encoded)?;
        std::fs::rename(temp_path, path)?;
        
        Ok(())
    }
}
```

### Option 4: Hybrid Approach (Index + Distribution Files)

Split metadata into multiple files with an index.

**Structure:**
```
~/.kopi/cache/
├── index.json              # Lightweight index
├── distributions/
│   ├── temurin.json       # Per-distribution data
│   ├── corretto.json
│   └── zulu.json
└── manifest.json          # Cache metadata
```

**Index Structure:**
```json
{
  "version": 1,
  "distributions": {
    "temurin": {
      "file": "temurin.json",
      "last_updated": "2024-01-20T10:00:00Z",
      "major_versions": [8, 11, 17, 21],
      "latest_lts": "21.0.1"
    }
  }
}
```

**Advantages:**
- Partial loading (only needed distributions)
- Parallel updates possible
- Natural sharding by distribution
- Human-readable individual files
- Simpler cache invalidation
- Good balance of performance and simplicity

**Disadvantages:**
- Multiple file operations
- Directory structure management
- Index consistency maintenance
- More complex than single file
- More filesystem overhead

**Implementation:**
```rust
use std::path::PathBuf;
use std::fs;

struct HybridCache {
    cache_dir: PathBuf,
}

impl HybridCache {
    fn load_distribution(&self, name: &str) -> Result<Distribution> {
        // Load index
        let index_path = self.cache_dir.join("index.json");
        let index_data = fs::read_to_string(&index_path)?;
        let index: Index = serde_json::from_str(&index_data)?;
        
        // Check if distribution exists
        let meta = index.distributions.get(name)
            .ok_or(Error::NotFound)?;
        
        // Load distribution file
        let path = self.cache_dir
            .join("distributions")
            .join(&meta.file);
        
        let data = fs::read_to_string(&path)?;
        let dist: Distribution = serde_json::from_str(&data)?;
        
        Ok(dist)
    }
    
    fn update_distribution(
        &self,
        name: &str,
        data: Distribution,
    ) -> Result<()> {
        // Ensure directories exist
        fs::create_dir_all(self.cache_dir.join("distributions"))?;
        
        // Write distribution file
        let path = self.cache_dir
            .join("distributions")
            .join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&path, json)?;
        
        // Update index
        let index_path = self.cache_dir.join("index.json");
        let mut index = if index_path.exists() {
            let data = fs::read_to_string(&index_path)?;
            serde_json::from_str(&data)?
        } else {
            Index::new()
        };
        
        index.distributions.insert(name.to_string(), DistributionMeta {
            file: format!("{}.json", name),
            last_updated: Utc::now(),
            major_versions: data.extract_major_versions(),
            latest_lts: data.find_latest_lts(),
        });
        
        // Save index
        let index_json = serde_json::to_string_pretty(&index)?;
        fs::write(&index_path, index_json)?;
        
        Ok(())
    }
}
```

## Comparison Matrix

| Aspect | JSON File | SQLite | Binary | Hybrid |
|--------|-----------|---------|---------|---------|
| **Simplicity** | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| **Performance** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Human Readable** | ⭐⭐⭐⭐⭐ | ❌ | ❌ | ⭐⭐⭐⭐⭐ |
| **Concurrent Access** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| **Partial Updates** | ❌ | ⭐⭐⭐⭐⭐ | ❌ | ⭐⭐⭐⭐ |
| **Query Flexibility** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| **Storage Efficiency** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Dependencies** | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Maintenance** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Offline Support** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

## Decision

We will implement a **phased approach**:

1. **Phase 1 (MVP)**: Single JSON file
2. **Phase 2 (Optimization)**: Hybrid approach
3. **Future**: Consider SQLite if complex queries needed

### Rationale

1. **Start Simple**: JSON file is the fastest to implement and debug
2. **Proven Path**: Can validate assumptions before optimization
3. **Natural Evolution**: Easy migration from JSON to hybrid
4. **Avoid Over-engineering**: SQLite is overkill for current needs
5. **Maintain Flexibility**: JSON format allows easy schema evolution

### Implementation Plan

#### Phase 1: JSON File (MVP)
```rust
// Core types matching foojay.io API
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataCache {
    pub version: u32,
    pub last_updated: DateTime<Utc>,
    pub distributions: HashMap<String, Distribution>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Distribution {
    pub name: String,
    pub display_name: String,
    pub versions: HashMap<String, Version>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Version {
    pub version: String,
    pub lts: bool,
    pub architectures: HashMap<String, Architecture>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Architecture {
    pub arch: String,
    pub os: String,
    pub download_url: String,
    pub checksum: String,
    pub checksum_type: String,
    pub size: u64,
}
```

#### Phase 2: Hybrid Approach (if needed)
- Keep same data structures
- Split into index + distribution files
- Load only required distributions
- Implement incremental updates
- Note: Only implement if single JSON file becomes a performance issue

### Migration Strategy

1. **Version field** in all formats for compatibility
2. **Automatic migration** on version mismatch
3. **Backward compatibility** for at least one version
4. **Clear error messages** for incompatible versions

### Recommended File Structure
```
~/.kopi/
├── cache/
│   ├── metadata.json          # Simple approach (MVP)
│   └── distributions/         # Hybrid approach (future)
│       ├── index.json
│       ├── temurin.json
│       └── corretto.json
├── config.toml
└── installed/
    └── jdks/
```

### Cache Management Strategy
- **No automatic expiration** - cache persists indefinitely
- **Explicit refresh only** - users control when to update
- `kopi cache refresh` command fetches latest metadata
- `kopi cache info` shows cache location and last updated time
- `kopi cache clear` removes cached data
- **Lazy loading** - fetch from API only when requested version not in cache

This approach is ideal for CLI tools because:
- JDK metadata rarely changes (new versions added ~monthly)
- Users explicitly choose when they need updates
- Maximizes offline capability
- Eliminates unnecessary network calls
- Simplifies implementation (no TTL logic needed)

## Consequences

### Positive
- Quick initial implementation
- Easy to debug and inspect
- Natural upgrade path
- Minimal dependencies
- Good performance for typical use

### Negative
- Initial implementation may need optimization
- JSON files larger than binary formats
- Full file loads initially required

### Mitigation
- Monitor cache file sizes
- Add metrics for performance
- Plan for hybrid migration early
- Document cache format clearly

## References
- [serde documentation](https://serde.rs/)
- [foojay.io API](https://api.foojay.io/swagger-ui/index.html)
- Similar tools: rustup, volta, nvm metadata handling