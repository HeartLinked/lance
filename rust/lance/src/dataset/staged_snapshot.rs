// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The Lance Authors

//! Compose several writes into one Transaction V2 commit, each building on
//! what the earlier ones staged.

use std::sync::Arc;

use arrow_array::RecordBatch;
use lance_index::{IndexParams, IndexType};
use lance_table::transaction::action::UserStep;

use crate::dataset::transaction::Transaction;
use crate::{Dataset, Result};

/// A fragment a [`StagedSnapshot`] added, before it has a committed id.
///
/// Opaque on purpose: the snapshot owns the operation's local tokens, so a
/// caller cannot name a fragment some other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedFragment {
    token: u32,
}

/// The read version plus a [`CompositeOperation`] still being composed, and a
/// [`Dataset`] over the speculative manifest that operation would produce.
///
/// Each write consumes the snapshot and returns the next one, so a later write
/// reads everything the earlier ones staged -- an index segment can cover
/// fragments appended in the same commit. Fragments the operation adds are
/// numbered by provisional id in the speculative manifest (see
/// [`provisional_fragment_id`]); a segment built over them stores those ids,
/// and the commit records the translation to the ids it allocates.
///
/// Experimental: committing a snapshot writes Transaction V2, with the
/// compatibility consequences described on
/// [`CommitBuilder::with_experimental_composite_operations`](crate::dataset::CommitBuilder::with_experimental_composite_operations).
///
/// [`CompositeOperation`]: lance_table::transaction::action::CompositeOperation
/// [`provisional_fragment_id`]: lance_table::transaction::action::provisional_fragment_id
pub struct StagedSnapshot {
    read: Arc<Dataset>,
    steps: Vec<UserStep>,
    next_fragment_token: u32,
    staged: Dataset,
}

impl StagedSnapshot {
    /// Start composing an operation that reads `read`.
    pub async fn try_new_experimental(read: Arc<Dataset>) -> Result<Self> {
        let _ = read;
        todo!()
    }

    /// The speculative dataset: the read version with every staged step
    /// applied. Not a committed version.
    pub fn dataset(&self) -> &Dataset {
        &self.staged
    }

    /// Write `batches` as new fragments.
    pub async fn append(
        self,
        description: impl Into<String>,
        batches: Vec<RecordBatch>,
    ) -> Result<(Self, Vec<StagedFragment>)> {
        let _ = (description.into(), batches);
        todo!()
    }

    /// Build a segment of the index `name` on `column` over `fragments`, which
    /// this snapshot added, and add it to that index.
    pub async fn add_index_segment(
        self,
        description: impl Into<String>,
        name: &str,
        column: &str,
        index_type: IndexType,
        params: &dyn IndexParams,
        fragments: &[StagedFragment],
    ) -> Result<Self> {
        let _ = (
            description.into(),
            name,
            column,
            index_type,
            params,
            fragments,
        );
        todo!()
    }

    /// The transaction to commit, reading the snapshot's read version.
    pub fn into_transaction(self) -> Transaction {
        todo!()
    }

    /// Commit the composed operation onto the latest version.
    pub async fn commit(self) -> Result<Dataset> {
        todo!()
    }
}
