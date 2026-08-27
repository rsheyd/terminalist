# SQLite Cache Design

## Goal

Make Terminalist Edge's SQLite database explicitly a **persistent but disposable cache** rather than a durable application datastore.

The cache should survive ordinary application restarts so Terminalist can:

- start quickly from cached data;
- remain useful during temporary network/API failures;
- avoid depending on a successful Todoist sync at startup.

At the same time, Terminalist should be able to safely discard and rebuild the database whenever its schema becomes incompatible.

## Design Principle

Todoist remains the authoritative source of task data.

SQLite exists only to improve availability and performance.

Therefore:

> Deleting `terminalist.db` must not cause permanent loss of user data.

Any feature that requires authoritative local-only state should either:

- persist that state somewhere explicitly durable and separate from the cache; or
- preferably, be redesigned so its state is represented remotely.

The proposed remote-backed Trash design is one example.

## Why Treat SQLite as a Cache?

Terminalist already needs a local SQLite file for fast access, offline viewing, search, and synchronization. Treating that file as a cache preserves those benefits while substantially simplifying its lifecycle.

### Simpler Schema Evolution

A durable database requires Terminalist to support upgrades from historical schemas.

Every schema change can otherwise require:

- a migration;
- migration ordering;
- upgrade-path testing;
- handling partially failed migrations;
- potentially maintaining compatibility with databases created by much older releases.

With a disposable cache, an incompatible schema simply causes the cache to be recreated and repopulated from Todoist.

Schema evolution becomes approximately:

```text
change schema
→ increment CACHE_SCHEMA_VERSION
→ rebuild cache on next launch
```

rather than maintaining an ever-growing migration history.

### Clear Source of Truth

Todoist remains authoritative, while SQLite contains a local representation of Todoist data.

This avoids situations where Terminalist must determine whether the local database or Todoist contains the canonical version of a task.

The ownership model becomes:

```text
Todoist
   ↓
authoritative user data
   ↓
Terminalist SQLite cache
```

Local mutations still update the cache immediately for responsive UI behavior, but ultimately synchronize with Todoist.

### Easier Recovery From Corruption

Because the cache is reconstructible, database corruption is substantially less dangerous.

Instead of attempting database repair or risking user-data loss, Terminalist can discard the damaged cache and perform a fresh synchronization.

### Easier Development

Developers can change cached representations without designing migration paths for every previous development or release schema.

This is especially useful while Terminalist is evolving quickly.

Developers can also safely remove `terminalist.db` when debugging storage or synchronization issues, knowing that the application should reconstruct its state.

### Simpler Testing

Tests do not need to validate every historical sequence of database migrations.

The important lifecycle becomes much smaller:

- create current cache;
- reopen compatible cache;
- reject/rebuild incompatible cache;
- reconstruct data from Todoist.

This reduces both test complexity and the number of storage states that Terminalist needs to support.

### Fewer Long-Term Compatibility Obligations

Once a database is treated as durable user storage, old versions effectively become part of the application's persistent data format.

Treating SQLite as a cache avoids making internal database structures part of Terminalist's long-term compatibility contract.

Terminalist can change its internal representation as needed as long as it can reconstruct the new representation from Todoist.

### Retains the Benefits of Persistence

"Disposable cache" does **not** mean recreating SQLite on every launch.

A compatible cache remains on disk and continues to provide:

- immediate startup data;
- offline/read-only availability;
- fast database-backed search;
- reduced unnecessary API work;
- incremental synchronization;
- continuity through temporary Todoist or network failures.

The cache is disposable in terms of **data ownership**, not expected lifetime.

## Current Behavior

Terminalist Edge currently retains `terminalist.db` between runs.

Schema initialization uses `CREATE ... IF NOT EXISTS`, and some schema changes are handled through lightweight migrations such as inspecting table columns and adding missing fields.

This effectively creates an evolving persistent database that must remain compatible across releases.

Over time, that implies maintaining:

- schema migrations;
- migration ordering;
- upgrade paths from older releases;
- recovery from partially failed migrations;
- tests for multiple historical database versions.

That complexity is unnecessary if the contents are reconstructible from Todoist.

## Proposed Behavior

### Normal Startup

On startup:

1. Locate the existing SQLite cache.
2. Determine its schema version.
3. Compare it with the schema version expected by the running Terminalist binary.
4. If the versions match, open and reuse the cache.
5. Immediately display cached data.
6. Start the normal Todoist synchronization in the background.

This preserves the fast/offline startup behavior.

### First Run

If no database exists:

1. Create a new database.
2. Initialize the current schema.
3. Store the current cache schema version.
4. Synchronize from Todoist.

### Schema Version Mismatch

If a database exists but its schema version does not match the application's expected version:

1. Close any connection to the old database.
2. Remove or replace the cache.
3. Create a fresh database using the current schema.
4. Store the current schema version.
5. Perform a full Todoist synchronization.

No migration between cache schemas is required.

## Schema Versioning

Maintain a single integer constant in the application, for example:

```text
CACHE_SCHEMA_VERSION = 3
```

Increment it whenever a release changes the SQLite schema in a way that could make an existing cache incompatible.

Good candidates include:

- adding/removing/renaming columns;
- changing column semantics;
- adding/removing tables;
- changing relationships or constraints;
- changing indexes where rebuilding is preferable;
- changing cached representations in ways old rows cannot safely satisfy.

Pure application changes that do not affect stored data do not require a version bump.

## Version Storage

Prefer SQLite's built-in:

```sql
PRAGMA user_version;
```

This provides a simple integer stored directly in the SQLite database without requiring a dedicated metadata table.

For example:

```sql
PRAGMA user_version = 3;
```

On startup Terminalist can query the existing value and compare it to `CACHE_SCHEMA_VERSION`.

## Cache Rebuild Safety

Cache rebuilding should be treated as an expected maintenance operation, not an error condition.

A version mismatch should therefore produce an informational log such as:

```text
Cache schema changed from 2 to 3; rebuilding local cache.
```

If the old database cannot be removed or replaced, Terminalist should surface a clear error rather than attempting to run against an incompatible schema.

## Offline Behavior

A schema-version mismatch creates one unavoidable edge case:

If Terminalist upgrades while the user is offline, the old cache may contain useful data but cannot necessarily be safely opened by the new binary.

The simplest initial policy is:

- rebuild the cache;
- if synchronization fails because the user is offline, show the normal empty/offline state.

This is acceptable if schema changes are relatively infrequent.

A future improvement could rename the incompatible database rather than immediately deleting it, allowing recovery or downgrade if necessary.

For example:

```text
terminalist.db
terminalist.db.old
```

This is optional and should not turn old caches into supported persistent state.

## Synchronization State

Some locally persisted values are operational rather than user data.

For example, Todoist incremental Sync API tokens may be stored in SQLite.

These can still live in the disposable cache because losing them only causes Terminalist to perform a full synchronization.

The rule remains:

> If losing a value causes extra work but no permanent user-data loss, it may live in the cache.

## Backend Configuration and Credentials

Backend records currently live in SQLite as well.

If the database becomes fully disposable, credentials or backend configuration that cannot be reconstructed automatically should not rely solely on the cache.

Terminalist should determine whether backend configuration should instead come from:

- environment variables;
- the existing configuration file;
- a separate durable local settings store.

Any required credential/configuration state should survive cache invalidation.

## Local-Only Features

Before fully adopting the cache model, audit SQLite fields and tables for data that cannot be reconstructed from Todoist.

Examples to check include:

- local Trash/tombstones;
- pending offline mutations;
- user annotations;
- undo history;
- AI-specific persistent data;
- UI preferences;
- backend credentials.

Each should be classified as either:

**Reconstructible**  
Safe to keep in the SQLite cache.

**Durable local state**  
Must move elsewhere or receive explicit migration guarantees.

The planned remote-backed Trash redesign should remove the most obvious current conflict.

## Removing Existing Migrations

Once the cache-version mechanism is in place, lightweight schema migrations used only to preserve cached Todoist data can be removed.

Instead of:

```text
old schema
    ↓
migration
    ↓
new schema
```

Terminalist will use:

```text
old cache schema
    ↓
invalidate
    ↓
new cache schema
    ↓
full sync
```

This should substantially simplify storage initialization.

## Multi-Process Considerations

Terminalist should avoid deleting a database file while another running process still has it open.

Schema invalidation should happen before the current process establishes its long-lived database connection.

If simultaneous Terminalist instances need to be supported, cache replacement behavior should be designed so one instance cannot invalidate another instance's active database unexpectedly.

At minimum, this behavior should be documented and covered by tests.

## Testing

Add tests covering:

- first startup creates the current schema/version;
- matching schema version reuses existing cached data;
- mismatched schema version rebuilds the cache;
- rebuilt cache contains the current schema;
- failed remote synchronization does not destroy a valid same-version cache;
- an incremental sync token can safely disappear during a rebuild;
- incompatible schemas are never opened as though they were current;
- any durable state identified by the audit survives cache invalidation through its separate storage mechanism.

## Migration From Current Edge

1. Audit SQLite-backed state and identify anything authoritative/local-only.
2. Move or redesign authoritative state as necessary.
3. Add `CACHE_SCHEMA_VERSION`.
4. Read/write `PRAGMA user_version`.
5. Detect mismatches before normal database initialization.
6. Recreate the cache on mismatch.
7. Remove lightweight migrations that only preserve reconstructible cached data.
8. Add cache lifecycle tests.
9. Document SQLite explicitly as a disposable cache.

Existing Edge users may lose their cached data once when this architecture is introduced. A subsequent Todoist sync should reconstruct it.

## Success Criteria

The transition is complete when:

- Terminalist opens an existing compatible cache across normal launches;
- cached data remains usable when startup synchronization fails;
- schema changes require only incrementing a cache schema version rather than writing migrations;
- incompatible caches are automatically recreated;
- rebuilding `terminalist.db` cannot permanently destroy user data;
- operational state such as sync tokens can safely disappear;
- authoritative local state is either stored separately or eliminated through remote-backed designs.