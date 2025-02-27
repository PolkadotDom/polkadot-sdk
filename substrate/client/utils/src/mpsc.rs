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

//! Code to meter unbounded channels.

pub use async_channel::{TryRecvError, TrySendError};

use crate::metrics::{
	DROPPED_LABEL, RECEIVED_LABEL, SENT_LABEL, UNBOUNDED_CHANNELS_COUNTER, UNBOUNDED_CHANNELS_SIZE,
};
use async_channel::{Receiver, Sender};
use futures::{
	stream::{FusedStream, Stream},
	task::{Context, Poll},
};
use log::error;
use sp_arithmetic::traits::SaturatedConversion;
use std::{
	backtrace::Backtrace,
	pin::Pin,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

/// Wrapper Type around [`async_channel::Sender`] that increases the global
/// measure when a message is added.
#[derive(Debug)]
pub struct TracingUnboundedSender<T> {
	inner: Sender<T>,
	name: &'static str,
	queue_size_warning: usize,
	warning_fired: Arc<AtomicBool>,
	creation_backtrace: Arc<Backtrace>,
}

// Strangely, deriving `Clone` requires that `T` is also `Clone`.
impl<T> Clone for TracingUnboundedSender<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			name: self.name,
			queue_size_warning: self.queue_size_warning,
			warning_fired: self.warning_fired.clone(),
			creation_backtrace: self.creation_backtrace.clone(),
		}
	}
}

/// Wrapper Type around [`async_channel::Receiver`] that decreases the global
/// measure when a message is polled.
#[derive(Debug)]
pub struct TracingUnboundedReceiver<T> {
	inner: Receiver<T>,
	name: &'static str,
}

/// Wrapper around [`async_channel::unbounded`] that tracks the in- and outflow via
/// `UNBOUNDED_CHANNELS_COUNTER` and warns if the message queue grows
/// above the warning threshold.
pub fn tracing_unbounded<T>(
	name: &'static str,
	queue_size_warning: usize,
) -> (TracingUnboundedSender<T>, TracingUnboundedReceiver<T>) {
	let (s, r) = async_channel::unbounded();
	let sender = TracingUnboundedSender {
		inner: s,
		name,
		queue_size_warning,
		warning_fired: Arc::new(AtomicBool::new(false)),
		creation_backtrace: Arc::new(Backtrace::force_capture()),
	};
	let receiver = TracingUnboundedReceiver { inner: r, name: name.into() };
	(sender, receiver)
}

impl<T> TracingUnboundedSender<T> {
	/// Proxy function to [`async_channel::Sender`].
	pub fn is_closed(&self) -> bool {
		self.inner.is_closed()
	}

	/// Proxy function to [`async_channel::Sender`].
	pub fn close(&self) -> bool {
		self.inner.close()
	}

	/// Proxy function to `async_channel::Sender::try_send`.
	pub fn unbounded_send(&self, msg: T) -> Result<(), TrySendError<T>> {
		self.inner.try_send(msg).inspect(|_| {
			UNBOUNDED_CHANNELS_COUNTER.with_label_values(&[self.name, SENT_LABEL]).inc();
			UNBOUNDED_CHANNELS_SIZE
				.with_label_values(&[self.name])
				.set(self.inner.len().saturated_into());

			if self.inner.len() >= self.queue_size_warning &&
				self.warning_fired
					.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
					.is_ok()
			{
				error!(
					"The number of unprocessed messages in channel `{}` exceeded {}.\n\
					 The channel was created at:\n{}\n
					 Last message was sent from:\n{}",
					self.name,
					self.queue_size_warning,
					self.creation_backtrace,
					Backtrace::force_capture(),
				);
			}
		})
	}

	/// The number of elements in the channel (proxy function to [`async_channel::Sender`]).
	pub fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<T> TracingUnboundedReceiver<T> {
	/// Proxy function to [`async_channel::Receiver`].
	pub fn close(&mut self) -> bool {
		self.inner.close()
	}

	/// Proxy function to [`async_channel::Receiver`]
	/// that discounts the messages taken out.
	pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
		self.inner.try_recv().inspect(|_| {
			UNBOUNDED_CHANNELS_COUNTER.with_label_values(&[self.name, RECEIVED_LABEL]).inc();
			UNBOUNDED_CHANNELS_SIZE
				.with_label_values(&[self.name])
				.set(self.inner.len().saturated_into());
		})
	}

	/// The number of elements in the channel (proxy function to [`async_channel::Receiver`]).
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	/// The name of this receiver
	pub fn name(&self) -> &'static str {
		self.name
	}
}

impl<T> Drop for TracingUnboundedReceiver<T> {
	fn drop(&mut self) {
		// Close the channel to prevent any further messages to be sent into the channel
		self.close();
		// The number of messages about to be dropped
		let count = self.inner.len();
		// Discount the messages
		if count > 0 {
			UNBOUNDED_CHANNELS_COUNTER
				.with_label_values(&[self.name, DROPPED_LABEL])
				.inc_by(count.saturated_into());
		}
		// Reset the size metric to 0
		UNBOUNDED_CHANNELS_SIZE.with_label_values(&[self.name]).set(0);
		// Drain all the pending messages in the channel since they can never be accessed,
		// this can be removed once https://github.com/smol-rs/async-channel/issues/23 is
		// resolved
		while let Ok(_) = self.inner.try_recv() {}
	}
}

impl<T> Unpin for TracingUnboundedReceiver<T> {}

impl<T> Stream for TracingUnboundedReceiver<T> {
	type Item = T;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
		let s = self.get_mut();
		match Pin::new(&mut s.inner).poll_next(cx) {
			Poll::Ready(msg) => {
				if msg.is_some() {
					UNBOUNDED_CHANNELS_COUNTER.with_label_values(&[s.name, RECEIVED_LABEL]).inc();
					UNBOUNDED_CHANNELS_SIZE
						.with_label_values(&[s.name])
						.set(s.inner.len().saturated_into());
				}
				Poll::Ready(msg)
			},
			Poll::Pending => Poll::Pending,
		}
	}
}

impl<T> FusedStream for TracingUnboundedReceiver<T> {
	fn is_terminated(&self) -> bool {
		self.inner.is_terminated()
	}
}

#[cfg(test)]
mod tests {
	use super::tracing_unbounded;
	use async_channel::{self, RecvError, TryRecvError};

	#[test]
	fn test_tracing_unbounded_receiver_drop() {
		let (tracing_unbounded_sender, tracing_unbounded_receiver) =
			tracing_unbounded("test-receiver-drop", 10);
		let (tx, rx) = async_channel::unbounded::<usize>();

		tracing_unbounded_sender.unbounded_send(tx).unwrap();
		drop(tracing_unbounded_receiver);

		assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
		assert_eq!(rx.recv_blocking(), Err(RecvError));
	}
}
