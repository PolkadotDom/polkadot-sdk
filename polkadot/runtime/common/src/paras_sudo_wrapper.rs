// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! A simple wrapper allowing `Sudo` to call into `paras` routines.

use alloc::boxed::Box;
use codec::Encode;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use polkadot_primitives::Id as ParaId;
use polkadot_runtime_parachains::{
	configuration, dmp, hrmp,
	paras::{self, AssignCoretime, ParaGenesisArgs, ParaKind},
	ParaLifecycle,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: configuration::Config + paras::Config + dmp::Config + hrmp::Config {}

	#[pallet::error]
	pub enum Error<T> {
		/// The specified parachain is not registered.
		ParaDoesntExist,
		/// The specified parachain is already registered.
		ParaAlreadyExists,
		/// A DMP message couldn't be sent because it exceeds the maximum size allowed for a
		/// downward message.
		ExceedsMaxMessageSize,
		/// A DMP message couldn't be sent because the destination is unreachable.
		Unroutable,
		/// Could not schedule para cleanup.
		CouldntCleanup,
		/// Not a parathread (on-demand parachain).
		NotParathread,
		/// Not a lease holding parachain.
		NotParachain,
		/// Cannot upgrade on-demand parachain to lease holding parachain.
		CannotUpgrade,
		/// Cannot downgrade lease holding parachain to on-demand.
		CannotDowngrade,
		/// There are more cores than supported by the runtime.
		TooManyCores,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule a para to be initialized at the start of the next session.
		///
		/// This should only be used for TESTING and not on PRODUCTION chains. It automatically
		/// assigns Coretime to the chain and increases the number of cores. Thus, there is no
		/// running coretime chain required.
		#[pallet::call_index(0)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_para_initialize(
			origin: OriginFor<T>,
			id: ParaId,
			genesis: ParaGenesisArgs,
		) -> DispatchResult {
			ensure_root(origin)?;

			let assign_coretime = genesis.para_kind == ParaKind::Parachain;

			polkadot_runtime_parachains::schedule_para_initialize::<T>(id, genesis)
				.map_err(|_| Error::<T>::ParaAlreadyExists)?;

			if assign_coretime {
				T::AssignCoretime::assign_coretime(id)?;
			}

			Ok(())
		}

		/// Schedule a para to be cleaned up at the start of the next session.
		#[pallet::call_index(1)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_para_cleanup(origin: OriginFor<T>, id: ParaId) -> DispatchResult {
			ensure_root(origin)?;
			polkadot_runtime_parachains::schedule_para_cleanup::<T>(id)
				.map_err(|_| Error::<T>::CouldntCleanup)?;
			Ok(())
		}

		/// Upgrade a parathread (on-demand parachain) to a lease holding parachain
		#[pallet::call_index(2)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_parathread_upgrade(
			origin: OriginFor<T>,
			id: ParaId,
		) -> DispatchResult {
			ensure_root(origin)?;
			// Para backend should think this is a parathread (on-demand parachain)...
			ensure!(
				paras::Pallet::<T>::lifecycle(id) == Some(ParaLifecycle::Parathread),
				Error::<T>::NotParathread,
			);
			polkadot_runtime_parachains::schedule_parathread_upgrade::<T>(id)
				.map_err(|_| Error::<T>::CannotUpgrade)?;
			Ok(())
		}

		/// Downgrade a lease holding parachain to an on-demand parachain
		#[pallet::call_index(3)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_parachain_downgrade(
			origin: OriginFor<T>,
			id: ParaId,
		) -> DispatchResult {
			ensure_root(origin)?;
			// Para backend should think this is a parachain...
			ensure!(
				paras::Pallet::<T>::lifecycle(id) == Some(ParaLifecycle::Parachain),
				Error::<T>::NotParachain,
			);
			polkadot_runtime_parachains::schedule_parachain_downgrade::<T>(id)
				.map_err(|_| Error::<T>::CannotDowngrade)?;
			Ok(())
		}

		/// Send a downward XCM to the given para.
		///
		/// The given parachain should exist and the payload should not exceed the preconfigured
		/// size `config.max_downward_message_size`.
		#[pallet::call_index(4)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_queue_downward_xcm(
			origin: OriginFor<T>,
			id: ParaId,
			xcm: Box<xcm::opaque::VersionedXcm>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(paras::Pallet::<T>::is_valid_para(id), Error::<T>::ParaDoesntExist);
			let config = configuration::ActiveConfig::<T>::get();
			dmp::Pallet::<T>::queue_downward_message(&config, id, xcm.encode()).map_err(|e| match e
			{
				dmp::QueueDownwardMessageError::ExceedsMaxMessageSize =>
					Error::<T>::ExceedsMaxMessageSize.into(),
				dmp::QueueDownwardMessageError::Unroutable => Error::<T>::Unroutable.into(),
			})
		}

		/// Forcefully establish a channel from the sender to the recipient.
		///
		/// This is equivalent to sending an `Hrmp::hrmp_init_open_channel` extrinsic followed by
		/// `Hrmp::hrmp_accept_open_channel`.
		#[pallet::call_index(5)]
		#[pallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_establish_hrmp_channel(
			origin: OriginFor<T>,
			sender: ParaId,
			recipient: ParaId,
			max_capacity: u32,
			max_message_size: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			hrmp::Pallet::<T>::init_open_channel(
				sender,
				recipient,
				max_capacity,
				max_message_size,
			)?;
			hrmp::Pallet::<T>::accept_open_channel(recipient, sender)?;
			Ok(())
		}
	}
}
