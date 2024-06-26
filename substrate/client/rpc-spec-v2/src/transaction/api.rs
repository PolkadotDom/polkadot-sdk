// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! API trait for transactions.

use crate::transaction::{error::ErrorBroadcast, event::TransactionEvent};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sp_core::Bytes;

#[rpc(client, server)]
pub trait TransactionApi<Hash: Clone> {
	/// Submit an extrinsic to watch.
	///
	/// See [`TransactionEvent`](crate::transaction::event::TransactionEvent) for details on
	/// transaction life cycle.
	///
	/// # Unstable
	///
	/// This method is unstable and subject to change in the future.
	#[subscription(
		name = "transactionWatch_v1_submitAndWatch" => "transactionWatch_v1_watchEvent",
		unsubscribe = "transactionWatch_v1_unwatch",
		item = TransactionEvent<Hash>,
	)]
	fn submit_and_watch(&self, bytes: Bytes);
}

#[rpc(client, server)]
pub trait TransactionBroadcastApi {
	/// Broadcast an extrinsic to the chain.
	///
	/// # Unstable
	///
	/// This method is unstable and subject to change in the future.

	#[method(name = "transaction_v1_broadcast", with_extensions)]
	async fn broadcast(&self, bytes: Bytes) -> RpcResult<Option<String>>;

	/// Broadcast an extrinsic to the chain.
	///
	/// # Unstable
	///
	/// This method is unstable and subject to change in the future.
	#[method(name = "transaction_v1_stop", with_extensions)]
	async fn stop_broadcast(&self, operation_id: String) -> Result<(), ErrorBroadcast>;
}
