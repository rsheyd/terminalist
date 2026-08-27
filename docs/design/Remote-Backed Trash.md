# Remote-Backed Trash Design

## Goal

Replace Terminalist Edge's current local-only Trash implementation with a Todoist-backed soft-delete mechanism.

The primary architectural goal is to preserve recoverable deletion without requiring Terminalist's SQLite database to contain authoritative user data.

After this change:

- **Todoist remains the authoritative source of task data.**
- **SQLite remains a persistent but disposable cache.**
- Deleting Terminalist's local database should not cause loss of recoverable Trash items.

## Current Behavior

Terminalist Edge currently implements Local Trash by retaining deleted task data in SQLite after the task has been deleted remotely.

Deleted tasks are retained for approximately 30 days and can be recreated from their cached fields.

This makes the SQLite database more than a cache: some information in it no longer exists remotely and therefore cannot be reconstructed after deleting/rebuilding the database.

That complicates cache invalidation and schema evolution.

## Proposed Behavior

Instead of immediately deleting a task from Todoist, Terminalist should perform a **soft delete backed by Todoist**.

### Delete

When the user deletes a task:

1. Move the task to a dedicated Todoist project, e.g. **Terminalist Trash**.
2. Preserve enough information to determine its original project/section.
3. Record when the task was trashed.
4. Display the task in Terminalist's existing Trash smart view.

The task continues to exist remotely and therefore survives local cache deletion or reconstruction.

### Restore

When restoring a task:

1. Retrieve its original project/section information.
2. Move it back to its original location.
3. Remove any Terminalist-specific Trash metadata.

If the original project or section no longer exists, fall back to a reasonable location such as the Todoist Inbox.

### Permanent Deletion

Trash items should be permanently deleted after a retention period, initially **30 days**.

Cleanup can occur opportunistically during startup or synchronization; Terminalist does not need a continuously running cleanup process.

A future option could allow users to configure the retention period.

## Metadata

Terminalist needs to preserve at least:

- original project
- original section, if applicable
- deletion timestamp

Prefer storing this information remotely so that Trash remains fully reconstructible from Todoist.

The exact representation should be determined based on what the Todoist API can support cleanly. Avoid introducing a new authoritative local metadata table solely for Trash restoration.

## Terminalist Trash Project

Terminalist should create the Trash project automatically when first needed.

Consider making its purpose obvious, for example:

**Terminalist Trash**

Terminalist should recognize this project during synchronization and use its contents to populate the Trash smart view.

Normal project/sidebar views should probably hide the Trash project to avoid exposing an implementation detail.

If the user manually deletes the Trash project from Todoist, Terminalist should recreate it the next time a task is soft-deleted rather than treating this as an application error.

## Cache Implications

After this change, Trash data becomes reconstructible from Todoist.

This allows Terminalist's SQLite database to retain a simple contract:

> SQLite is a persistent optimization, not an authoritative data store.

Terminalist can therefore:

- retain SQLite between normal launches for fast/offline startup;
- invalidate the database when its schema version changes;
- recreate it from Todoist instead of maintaining migrations for cached application data.

This complements the proposed schema-version-based cache invalidation model.

## Migration

Existing Local Trash entries require a decision.

Because this feature is currently experimental/Edge functionality, the simplest migration may be:

1. Leave existing local Trash entries untouched until the new implementation ships.
2. Optionally migrate recoverable entries into the new remote Trash project.
3. Once remote-backed Trash is active, remove the local tombstone-specific behavior and associated schema fields/migrations.

If preserving existing Edge Trash contents is not important, simply dropping the old local Trash state is also reasonable.

## Open Questions

- What Todoist-supported mechanism should store original project/section and deletion timestamp?
- Should the Trash project itself be hidden from Terminalist's normal project list?
- Should tasks remain active while in Trash, or is there a better Todoist representation?
- What happens when the original project or section no longer exists?
- Should permanent deletion happen automatically after exactly 30 days or only during subsequent syncs?
- Should users be able to manually empty Trash?
- Should the retention period eventually be configurable?

## Success Criteria

The redesign is complete when:

- deleting a task in Terminalist remains recoverable for the retention period;
- Trash survives deletion/recreation of `terminalist.db`;
- restoring a task does not depend on authoritative local-only state;
- Trash can be reconstructed entirely from Todoist;
- the SQLite database can safely be treated as a disposable, versioned cache.