# JSON Index

The JSON index indexes a single path within a JSON column. It is a wrapper: it
decodes the value at that path out of each JSONB document, converts the decoded
values into one Arrow array, and hands that array to an ordinary scalar index —
the *target index* — which does all of the storage and searching.

A JSON value at a given path has no fixed type, so the wrapper has to pick one
Arrow type for the whole column before the target index can be built:

| JSONB type tag    | `JsonTargetDataType`                 | Value stored in the target index      |
|-------------------|--------------------------------------|---------------------------------------|
| `Boolean`         | `JSON_TARGET_DATA_TYPE_BOOLEAN`      | The decoded boolean                   |
| `Int64`           | `JSON_TARGET_DATA_TYPE_INT64`        | The decoded integer                   |
| `Float64`         | `JSON_TARGET_DATA_TYPE_FLOAT64`      | The decoded float                     |
| `String`          | `JSON_TARGET_DATA_TYPE_UTF8`         | The decoded string, unquoted          |

A JSON null, or a document whose path is absent, is indexed as a null.

The chosen type is recorded in the index details as `target_data_type`. It is
`JSON_TARGET_DATA_TYPE_UNSPECIFIED` for an index written before the details
carried the type, and for one whose type a later rewrite of the details could
not recover.

### Row Domain of the Target

JSON-path indexes do not support an Inverted target (create an Inverted index
directly on the JSON column instead), and a vector index is not a scalar-index
plugin, so it can never be wrapped either. A JSON index can otherwise wrap any
scalar target whose `basic_trainer` accepts the extracted Arrow type,
including BTree, Bitmap, LabelList, ZoneMap, BloomFilter, FM, NGram, R-Tree,
and MinHash LSH.

Unlike a standalone index, a JSON-wrapped target has no `IndexMetadata` entry
of its own to record a version in, and most target types do not otherwise
self-describe their version inside their own files: BTree, for example, is
simply told its domain by the version number its caller passes in, rather
than reading it back from `page_lookup.lance` or `page_data.lance` (MinHash
LSH, which stamps `minhash_lsh_index_version` into its own file metadata, is
the one exception). So the target's own version has to be recorded somewhere
in `JsonIndexDetails` itself: a new field, `target_index_version`, holds the
version the target was actually built at -- the same number that would sit in
`IndexMetadata::index_version` for a standalone instance of that type.
`target_index_version` is absent on a JSON index built before this field
existed; a reader then falls back to the legacy, per-target-type floor
version it already applies today: the row-id-domain floor version for BTree,
Bitmap, and LabelList (the only target types a pre-existing JSON index could
have trained in the address domain without recording it), and the target
plugin's one and only version for every other target type.

`target_data_type` completes the description of the *extracted values*;
`target_index_version` completes the description of the target index itself,
so the two fields together make `JsonIndexDetails` a lossless description of
which on-disk format the wrapped target uses. This also lifts the
restriction under which a JSON-wrapped BTree, Bitmap, or LabelList target
could not be created or optimized in the row-address domain on a
stable-row-id dataset: once the version is recorded explicitly, there is
nothing left to infer.

`target_index_version` does not affect `JSON_INDEX_VERSION` itself, which
tracks the JSON wrapper's own on-disk format and is unrelated to the target's
version; nor does it change how an old reader decides whether it can safely
skip a JSON index whose target it does not understand. That decision is
based on the target plugin's own declared maximum version, looked up by
decoding `target_details`' `type_url`, exactly as query routing and
`load_index` already do -- not on `JSON_INDEX_VERSION`, which says nothing
about the target type and is unaffected by a target's own version changes.

## Index Details

```protobuf
%%% proto.message.JsonIndexDetails %%%
```

The target index is identified by the `type_url` of `target_details`, which is
read to select the target index implementation before `target_details` itself is
decoded as that implementation's own details message.

## Storage Layout

The JSON index writes no files of its own. Its files are exactly the files of
its target index, written with that index's names and schemas into the same
index directory, and the target's format documentation describes them: for
example [BTree](btree.md) or [Bitmap](bitmap.md).

Reader navigation therefore has two steps. Decode `JsonIndexDetails` to recover
`path` and `target_details`, select the target implementation from the
`type_url` of `target_details`, then navigate the target index exactly as a
standalone index of that type, using `target_details` as its details and
`target_index_version` (see [Row Domain of the Target](#row-domain-of-the-target))
as its version.

## Accelerated Queries

Only the typed accessor functions are routed to a JSON index, and only when
their path argument is a literal equal to the indexed `path`:

| Function          | Value type evaluated as |
|-------------------|-------------------------|
| `json_get_bool`   | `BOOLEAN`               |
| `json_get_int`    | `INT64`                 |
| `json_get_float`  | `FLOAT64`               |
| `json_get_string` | `UTF8`                  |

Once a predicate is routed, the query types the index can accelerate, and
whether the answer is exact, are those of the target index.

`json_extract` is deliberately not routed. It evaluates to serialized JSON text
while the target index holds decoded native values, so an indexed
`json_extract` predicate would answer a different question than an unindexed
one; quoting is also not order-preserving, so even a `UTF8` target cannot serve
its ranges.

For example, given the documents `{"val": "ab"}` and `{"val": "ab!"}`, a JSON
index on `val` stores the `UTF8` keys `ab` and `ab!`, while
`json_extract(doc, 'val')` evaluates to the text `"ab"` and `"ab!"`, quotes
included:

- `json_extract(doc, 'val') = '"ab"'` matches the first document, but the target
  index holds no key `"ab"`, so an indexed lookup would match nothing.
- `json_extract(doc, 'val') > '"ab"'` does not match the second document, because
  `!` sorts before `"`. In the target index `ab!` sorts after `ab`, so an indexed
  range would match it.
