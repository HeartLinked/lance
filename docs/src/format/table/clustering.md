# Clustering Metadata

Clustering metadata lets providers persist their configuration and record which
clustering definition was used to organize a fragment or index segment.

## What Lance Stores

### Table Configuration

A table can retain multiple clustering definitions. Different fragments and
index segments can reference different definitions.

`Manifest.clustering` is a repeated list of immutable definitions. Each contains:

- `id`: a unique, non-zero table-local ID, not a position in the list.
- `columns`: the ordered, non-empty list of distinct clustering column field IDs.
- `provider`: a unique, non-empty, case-sensitive identifier for the clustering
  implementation, for example `com.example.zorder`.
- `version`: a non-zero version assigned by the provider to these layout rules.

The ID identifies the whole definition, including its provider and version.

`Manifest.max_clustering_id` records the highest ID ever allocated in this
lineage, or 0 before the first allocation. A new definition gets
`max_clustering_id + 1`, starting at 1, and updates the value in the same commit.
Allocate against the latest manifest on each commit attempt. Removing definitions
or restoring an older snapshot must not lower the value or make IDs reusable.
Every definition's ID must be at most this value; `2^32 - 1` exhausts the ID space.

Providers may store extra information in `Manifest.table_metadata`, using keys
such as `<provider>.<clustering_id>.<key>`. Each provider defines its keys and values.

#### Allocating Definitions

A writer allocates definitions with the `UpdateConfig` transaction operation
(`new_clustering_definitions`, see [Transactions](transaction.md#updateconfig)).
The writer chooses the ids explicitly from the `max_clustering_id` it read, and
the commit verifies that each id is exactly one above the mark of the manifest
the transaction is applied to, in order. If a concurrent commit moved the mark,
the transaction fails with a non-retryable commit conflict: the ids, and any
`table_metadata` keys that embed them, were computed against a stale mark and
must be regenerated. Because `table_metadata_updates` is part of the same
operation, a definition and the provider metadata that refers to it are
committed in one version; readers never see one without the other.

Allocation never modifies or removes an existing definition. Dropping or
restoring follows the rules above; the mark only moves up.

### Fragment and Index Segment Information

Each fragment and index segment may set `clustering_id` to a definition in the
same manifest.

Providers can store extra per-object information in the generic metadata maps. See
[Fragment Metadata](index.md#fragment-metadata) and
[Index Segment Metadata](../index/index.md#index-segment-metadata) for their
definitions and maintenance rules.

## Reader and Writer Compatibility

Set `FLAG_OBJECT_METADATA` (`1 << 17`) in a snapshot's `writer_feature_flags`
if it contains any of the following:

- Table clustering definitions or a nonzero `max_clustering_id`.
- Fragment clustering IDs, including records in a fragment metadata tree.
- Index segment clustering IDs, including records in the index section.

This prevents older writers from dropping information they do not understand.
No clustering reader flag is required: readers can ignore clustering metadata
and use valid statistics and index coverage for pruning. Nonempty generic metadata
maps require the same flag; see
[Fragment Metadata](index.md#fragment-metadata) and
[Index Segment Metadata](../index/index.md#index-segment-metadata).

### Legacy Column-Level Hints

`Field.unenforced_clustering_key_position` and `Field.unenforced_clustering_key`
are **legacy column-level hints**. Keep their existing rules. Do not remove them
or change them to match the new configuration.
