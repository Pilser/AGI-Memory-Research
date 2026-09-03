# HelixDB Embedded Integration — MNEMOS Reference

> How the new HelixDB 3.0.0 Rust SDK with embedded execution replaces the old `helix_rs` HTTP approach

---

## Overview

The old MNEMOS research used `helix_rs` — a now-obsolete crate that sent named queries over HTTP to a separately-running HelixDB server. The new approach uses `helix-db` 3.0.0 which provides:

- **Dynamic query building** — schema and queries are Rust code, defined and compiled in-process
- **Embedded execution** — the database runs inside your process, no HTTP server needed
- **Multi-backend storage** — disk (default), in-memory, or object storage

This document covers the integration layer: setup, client initialization, storage backends, and cache configuration.

---

## SDK Setup

### Cargo.toml

```toml
[dependencies]
helix-db = { version = "3.0.0", features = ["embedded"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Or via CLI:

```bash
cargo add helix-db@3.0.0 --features embedded
```

### Feature Flags

| Feature | Purpose |
|---------|---------|
| `embedded` | Enables `HelixDbSource` and `Client::open()` for in-process execution |
| (default) | HTTP client only — `Client::new("http://...")` for server mode |

For MNEMOS, you always want `embedded`. The HTTP client is available without the feature if you need both.

---

## Client Initialization

### Writer Handle (Read + Write)

The default for MNEMOS — needs to read, write, and mutate:

```rust
use helix_db::{Client, HelixDbSource};

// Disk (default for production)
let client = Client::open(HelixDbSource::Disk {
    root: "/data/helix".into(),
    database: "mnemos".to_string(),
}).await?;

// In-memory (for tests)
let client = Client::open(HelixDbSource::InMemory {
    database: "mnemos".to_string(),
}).await?;

// Object storage (for durable shared storage)
let client = Client::open(HelixDbSource::ObjectStorage {
    database: "mnemos".to_string(),
    bucket: "helix-production".into(),
    region: "eu-west-2".into(),
}).await?;
```

### Read-Only Handle

For processes that must never mutate (e.g., read-only replicas, audit tools):

```rust
let reader = Client::open_reader(HelixDbSource::Disk {
    root: "/data/helix".into(),
    database: "mnemos".to_string(),
}).await?;
```

Read-only handles reject write requests at the SDK level — no gateway needed.

### Old vs New Comparison

| Aspect | Old (`helix_rs`) | New (`helix-db` 3.0.0) |
|--------|------------------|----------------------|
| Import | `use helix_rs::{HelixDB, HelixDBClient}` | `use helix_db::{Client, HelixDbSource}` |
| Init | `HelixDB::new(Some("http://localhost"), Some(6969), None)` | `Client::open(HelixDbSource::Disk { ... }).await` |
| Server required | Yes — separate HelixDB process | No — embedded in-process |
| Query call | `client.query("QueryName", &json!({...})).await` | `client.query(request).send().await` |
| Query definition | Separate `.hx` files | `#[query]` functions in Rust |

---

## Storage Backends

### Disk (Default)

Persistent storage on the local filesystem. Data survives restarts. This is the default for production MNEMOS deployments.

```rust
HelixDbSource::Disk {
    root: "/data/helix".into(),      // filesystem path for all database files
    database: "mnemos".to_string(),   // logical database name
}
```

**When to use:** Production deployments, anything that must persist.

**Characteristics:**
- Data written to `root/database/` on the filesystem
- Survives process restarts and system reboots
- Supports full cache configuration (memory + hybrid profiles)
- Default flush interval: 3ms (batching delay before disk I/O)

### In-Memory

Process-local, non-persistent storage. Data is lost on process exit.

```rust
HelixDbSource::InMemory {
    database: "mnemos".to_string(),
}
```

**When to use:** Unit tests, integration tests, development prototyping, CI pipelines.

**Characteristics:**
- Fastest option — no disk I/O
- Zero persistence — data gone when process exits
- Default flush interval: 1ms
- Vector memory: 64 MiB default

### Object Storage

Durable shared storage backed by an S3-compatible service.

```rust
HelixDbSource::ObjectStorage {
    database: "mnemos".to_string(),
    bucket: "helix-production".into(),
    region: "eu-west-2".into(),
}
```

**When to use:** Multi-instance deployments, cloud-native setups, when you need durable storage shared across processes.

**Characteristics:**
- Durable across restarts and instances
- Shared across multiple processes (with appropriate coordination)
- Default flush interval: 100ms
- Larger default caches: 256 MiB vector memory, 512 MiB SlateDB block
- Background startup warming enabled

---

## Cache Configuration

Cache settings are fixed when the handle opens — they cannot be changed without reopening the database.

### Default Local Profiles

| Setting | In-Memory | Disk | Object Storage |
|---------|-----------|------|----------------|
| Durable-write flush interval | 1 ms | 3 ms | 100 ms |
| Vector memory | 64 MiB | 64 MiB | 256 MiB |
| SlateDB block / metadata | 16 MiB / 8 MiB | 48 MiB / 16 MiB | 512 MiB / 128 MiB |
| Full-text search | 16 MiB | 16 MiB | 64 MiB |
| SlateDB / FTS startup warming | Off | Off | Background |
| Disk cache | Disabled | Disabled | Disabled |

All local profiles use a 16 MiB L0 target and 64 MiB unflushed-data ceiling.

### Explicit Cache Configuration

Pass a `CacheConfig` as the second argument to `Client::open()`:

```rust
use helix_db::{Client, HelixDbSource, CacheConfig, CacheMode};

// Memory-only caches (default behavior, explicit)
let client = Client::open(
    HelixDbSource::Disk {
        root: "/data/helix".into(),
        database: "mnemos".to_string(),
    },
    Some(CacheConfig {
        vector_memory_bytes: 512 * 1024 * 1024,  // 512 MiB
        mode: CacheMode::Memory,
    }),
).await?;

// Hybrid: memory + bounded disk caches
let client = Client::open(
    HelixDbSource::Disk {
        root: "/data/helix".into(),
        database: "mnemos".to_string(),
    },
    Some(CacheConfig {
        vector_memory_bytes: 512 * 1024 * 1024,  // 512 MiB
        mode: CacheMode::Hybrid {
            slate_memory_bytes: 256 * 1024 * 1024,
            slate_disk_path: "/var/cache/helix/slate".into(),
            slate_disk_bytes: 4 * 1024 * 1024 * 1024,  // 4 GiB
            object_store_disk_path: "/var/cache/helix/objects".into(),
            object_store_disk_bytes: 8 * 1024 * 1024 * 1024,  // 8 GiB
        },
    }),
).await?;
```

**Important:** Cache byte budgets bound those specific caches. They are NOT a hard cap on total process RSS — requests and index work can allocate transient memory.

---

## Executing Queries

### Basic Pattern

```rust
// Build a request using the DSL
let request = write_batch()
    .var_as("engram", g().add_n("Engram", vec![("name", "Alice")]))
    .returning(["engram"]);

// Execute
let response: serde_json::Value = client.query(request).send().await?;
```

### Read vs Write Requests

```rust
// Read request
let read_request = read_batch()
    .var_as("engrams", g().n_with_label("Engram").value_map(Some(vec!["$id", "name"])))
    .returning(["engrams"]);

let response = client.query(read_request).send().await?;

// Write request
let write_request = write_batch()
    .var_as("engram", g().add_n("Engram", vec![("name", "Alice")]))
    .returning(["engram"]);

let response = client.query(write_request).send().await?;
```

### Error Handling

```rust
match client.query(request).send().await {
    Err(error) if error.remote_code() == Some("index_not_found") => {
        // Create the index, wait for it to become active, then retry
    }
    Err(error) => {
        eprintln!("Query failed: {} — {}",
            error.remote_code().unwrap_or("unknown"),
            error.remote_message().unwrap_or("no details")
        );
    }
    Ok(response) => {
        println!("{response}");
    }
}
```

Error types available in the new SDK:

| Method | Returns |
|--------|---------|
| `error.remote_code()` | Stable error code (e.g., `"query_timeout"`, `"index_not_found"`) |
| `error.remote_message()` | Human-readable diagnostic |
| `error.status_code()` | HTTP status (when using HTTP client) |
| `error.raw_response_body()` | Full raw response for debugging |

---

## Embedded Request Rules

When using embedded mode (vs server mode), these constraints apply:

1. **Same contract** — `QueryRequest` and response types are identical to server mode
2. **Writer handles** can execute both read and write requests
3. **Read-only handles** reject write requests at the SDK level
4. **No gateway options** — writer-only, warm-only, API headers, and durability headers are rejected because there's no routing gateway
5. **Reopen to retain data** — reopen the same disk or object-storage source to retain canonical data across process lifetimes

---

## Migration Notes from Old Approach

### What Changed

| Old Pattern | New Pattern |
|-------------|-------------|
| `HelixDB::new(Some("http://localhost"), Some(6969), None)` | `Client::open(HelixDbSource::Disk { ... }).await` |
| `client.query("CreateEngram", &json!({...})).await` | `client.query(request).send().await` where `request` is built from DSL |
| Schema in `schema.hx` files | Schema created via `add_n()` / `add_e()` in Rust |
| Queries in `queries.hx` files | `#[query]` functions in Rust |
| Separate HelixDB server process | Embedded — no server needed |
| `helix_rs` crate | `helix-db` 3.0.0 crate |

### What Stayed the Same

- The graph data model: nodes, edges, properties, labels
- Vector search, full-text search, secondary indexes
- Transaction scope: one transaction per query invocation
- The request envelope: `request_type`, `query_name`, `query` batch with entries and returns

### What's New

- **Dynamic queries** — build queries programmatically in Rust, no separate query files needed
- **Parameters** — `#[query]` macro converts typed function arguments into request parameters
- **Type safety** — Rust and TypeScript reject `add_n` in a read batch at compile time
- **Embedded execution** — no network overhead, no server to manage
- **Storage flexibility** — disk, memory, or object storage via config

---

## Next Steps

- [Schema Mapping](./helixdb-embedded-schema.md) — how the old `schema.hx` maps to new Rust DSL
- [Query Conversion](./helixdb-embedded-queries.md) — every old query converted to `#[query]` functions
- [Retrieval Layer](./helixdb-embedded-retrieval.md) — CRR and Stimulation Layer with new SDK
- [Full Pipeline](./helixdb-embedded-pipeline.rs) — complete pipeline reference implementation
