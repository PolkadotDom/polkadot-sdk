// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Storage migrations for the `pallet-conviction-voting`.

use super::*;
use frame_support::{
    pallet_prelude::*,
    storage_alias,
    traits::UncheckedOnRuntimeUpgrade,
};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

mod v0 {
    use super::*;

    /// V0 type for [`crate::Params`].
    #[storage_alias]
    pub type Params<T: Config<I>, I: 'static> =
        StorageValue<Pallet<T, I>, ParamsOf<T, I>, ValueQuery>;
}

mod v1 {
    use super::*;

    pub struct MigrateToV1<T, I = ()>(PhantomData<(T, I)>);
    impl<T: Config<I>, I: 'static> UncheckedOnRuntimeUpgrade for MigrateToV1<T, I> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            Ok(Default::default())
        }

        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let mut transactions = 0;

            // VotingFor stores block number as a PriorLock.

            // ClassLocksFor does not appear to have any moment information.

            T::DbWeight::get().reads_writes(transactions, transactions)
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
            Ok(())
        }
    }
}

/// [`UncheckedOnRuntimeUpgrade`] implementation [`MigrateToV1`] wrapped in a
/// [`VersionedMigration`](frame_support::migrations::VersionedMigration), which ensures that:
/// - The migration only runs once when the on-chain storage version is 0
/// - The on-chain storage version is updated to `1` after the migration executes
/// - Reads/Writes from checking/settings the on-chain storage version are accounted for
pub type MigrateV0ToV1<T, I> = frame_support::migrations::VersionedMigration<
    0, // The migration will only execute when the on-chain storage version is 0
    1, // The on-chain storage version will be set to 1 after the migration is complete
    v1::MigrateToV1<T, I>,
    crate::pallet::Pallet<T, I>,
    <T as frame_system::Config>::DbWeight,
>;