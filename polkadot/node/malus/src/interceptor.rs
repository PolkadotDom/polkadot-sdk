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

//! A small set of wrapping types to cover most of our adversary test cases.
//!
//! This allows types with internal mutability to synchronize across
//! multiple subsystems and intercept or replace incoming and outgoing
//! messages on the overseer level.

use polkadot_node_subsystem::*;
pub use polkadot_node_subsystem::{messages::*, overseer, FromOrchestra};
use std::{collections::VecDeque, future::Future, pin::Pin};

/// Filter incoming and outgoing messages.
pub trait MessageInterceptor<Sender>: Send + Sync + Clone + 'static
where
	Sender: overseer::SubsystemSender<<Self::Message as overseer::AssociateOutgoing>::OutgoingMessages>
		+ Clone
		+ 'static,
{
	/// The message type the original subsystem handles incoming.
	type Message: overseer::AssociateOutgoing + Send + 'static;

	/// Filter messages that are to be received by
	/// the subsystem.
	///
	/// For non-trivial cases, the `sender` can be used to send
	/// multiple messages after doing some additional processing.
	fn intercept_incoming(
		&self,
		_sender: &mut Sender,
		msg: FromOrchestra<Self::Message>,
	) -> Option<FromOrchestra<Self::Message>> {
		Some(msg)
	}

	/// Specifies if we need to replace some outgoing message with another (potentially empty)
	/// message
	fn need_intercept_outgoing(
		&self,
		_msg: &<Self::Message as overseer::AssociateOutgoing>::OutgoingMessages,
	) -> bool {
		false
	}
	/// Send modified message instead of the original one
	fn intercept_outgoing(
		&self,
		_msg: &<Self::Message as overseer::AssociateOutgoing>::OutgoingMessages,
	) -> Option<<Self::Message as overseer::AssociateOutgoing>::OutgoingMessages> {
		None
	}
}

/// A sender with the outgoing messages filtered.
#[derive(Clone)]
pub struct InterceptedSender<Sender, Fil> {
	inner: Sender,
	message_filter: Fil,
}

#[async_trait::async_trait]
impl<OutgoingMessage, Sender, Fil> overseer::SubsystemSender<OutgoingMessage> for InterceptedSender<Sender, Fil>
where
	OutgoingMessage: overseer::AssociateOutgoing + Send + 'static + TryFrom<overseer::AllMessages>,
	Sender: overseer::SubsystemSender<OutgoingMessage>
		+ overseer::SubsystemSender<
				<
					<Fil as MessageInterceptor<Sender>>::Message as overseer::AssociateOutgoing
				>::OutgoingMessages
			>,
	Fil: MessageInterceptor<Sender>,
	<Fil as MessageInterceptor<Sender>>::Message: overseer::AssociateOutgoing,
	<
		<Fil as MessageInterceptor<Sender>>::Message as overseer::AssociateOutgoing
	>::OutgoingMessages:
		From<OutgoingMessage> + Send + Sync,
	<OutgoingMessage as TryFrom<overseer::AllMessages>>::Error: std::fmt::Debug,
{
	async fn send_message(&mut self, msg: OutgoingMessage) {
		self.send_message_with_priority::<overseer::NormalPriority>(msg).await;
	}

	async fn send_message_with_priority<P: Priority>(&mut self, msg: OutgoingMessage) {
		let msg = <
					<<Fil as MessageInterceptor<Sender>>::Message as overseer::AssociateOutgoing
				>::OutgoingMessages as From<OutgoingMessage>>::from(msg);
		if self.message_filter.need_intercept_outgoing(&msg) {
			if let Some(msg) = self.message_filter.intercept_outgoing(&msg) {
				self.inner.send_message(msg).await;
			}
		}
		else {
			self.inner.send_message(msg).await;
		}
	}

	fn try_send_message(
		&mut self,
		msg: OutgoingMessage,
	) -> Result<(), polkadot_node_subsystem_util::metered::TrySendError<OutgoingMessage>> {
		self.try_send_message_with_priority::<overseer::NormalPriority>(msg)
	}

	fn try_send_message_with_priority<P: Priority>(&mut self, msg: OutgoingMessage) -> Result<(), TrySendError<OutgoingMessage>> {
		let msg = <
				<<Fil as MessageInterceptor<Sender>>::Message as overseer::AssociateOutgoing
			>::OutgoingMessages as From<OutgoingMessage>>::from(msg);
		if self.message_filter.need_intercept_outgoing(&msg) {
			if let Some(real_msg) = self.message_filter.intercept_outgoing(&msg) {
				let orig_msg : OutgoingMessage = msg.into().try_into().expect("must be able to recover the original message");
				self.inner.try_send_message(real_msg).map_err(|e| {
					match e {
						TrySendError::Full(_) => TrySendError::Full(orig_msg),
						TrySendError::Closed(_) => TrySendError::Closed(orig_msg),
					}
				})
			}
			else {
				// No message to send after intercepting
				Ok(())
			}
		}
		else {
			let orig_msg : OutgoingMessage = msg.into().try_into().expect("must be able to recover the original message");
			self.inner.try_send_message(orig_msg)
		}
	}

	async fn send_messages<T>(&mut self, msgs: T)
	where
		T: IntoIterator<Item = OutgoingMessage> + Send,
		T::IntoIter: Send,
	{
		for msg in msgs {
			self.send_message(msg).await;
		}
	}

	fn send_unbounded_message(&mut self, msg: OutgoingMessage) {
		let msg = <
				<<Fil as MessageInterceptor<Sender>>::Message as overseer::AssociateOutgoing
			>::OutgoingMessages as From<OutgoingMessage>>::from(msg);
		if self.message_filter.need_intercept_outgoing(&msg) {
			if let Some(msg) = self.message_filter.intercept_outgoing(&msg) {
				self.inner.send_unbounded_message(msg);
			}
		}
		else {
			self.inner.send_unbounded_message(msg);
		}
	}
}

/// A subsystem context, that filters the outgoing messages.
pub struct InterceptedContext<Context, Fil>
where
	Context: overseer::SubsystemContext<Error=SubsystemError, Signal=OverseerSignal>,
	Fil: MessageInterceptor<<Context as overseer::SubsystemContext>::Sender>,
	<Context as overseer::SubsystemContext>::Sender:
		overseer::SubsystemSender<
			<
				<
					Fil as MessageInterceptor<<Context as overseer::SubsystemContext>::Sender>
				>::Message as overseer::AssociateOutgoing
			>::OutgoingMessages,
		>,
{
	inner: Context,
	message_filter: Fil,
	sender: InterceptedSender<<Context as overseer::SubsystemContext>::Sender, Fil>,
	message_buffer: VecDeque<FromOrchestra<<Context as overseer::SubsystemContext>::Message>>,
}

impl<Context, Fil> InterceptedContext<Context, Fil>
where
	Context: overseer::SubsystemContext<Error=SubsystemError,Signal=OverseerSignal>,
	Fil: MessageInterceptor<
		<Context as overseer::SubsystemContext>::Sender,
		Message = <Context as overseer::SubsystemContext>::Message,
	>,
	<Context as overseer::SubsystemContext>::Message: overseer::AssociateOutgoing,
	<Context as overseer::SubsystemContext>::Sender: overseer::SubsystemSender<
		<<Context as overseer::SubsystemContext>::Message as overseer::AssociateOutgoing>::OutgoingMessages
	>
{
	pub fn new(mut inner: Context, message_filter: Fil) -> Self {
		let sender = InterceptedSender::<<Context as overseer::SubsystemContext>::Sender, Fil> {
			inner: inner.sender().clone(),
			message_filter: message_filter.clone(),
		};
		Self { inner, message_filter, sender, message_buffer: VecDeque::new() }
	}
}

#[async_trait::async_trait]
impl<Context, Fil> overseer::SubsystemContext for InterceptedContext<Context, Fil>
where
	Context: overseer::SubsystemContext<Error=SubsystemError,Signal=OverseerSignal>,
	<Context as overseer::SubsystemContext>::Message:
		overseer::AssociateOutgoing,
	<Context as overseer::SubsystemContext>::Sender:
		overseer::SubsystemSender<
			<<Context as overseer::SubsystemContext>::Message as overseer::AssociateOutgoing>::OutgoingMessages
		>,
	InterceptedSender<<Context as overseer::SubsystemContext>::Sender, Fil>:
		overseer::SubsystemSender<
			<<Context as overseer::SubsystemContext>::Message as overseer::AssociateOutgoing>::OutgoingMessages
		>,
	Fil: MessageInterceptor<
		<Context as overseer::SubsystemContext>::Sender,
		Message = <Context as overseer::SubsystemContext>::Message,
	>,
{
	type Message = <Context as overseer::SubsystemContext>::Message;
	type Sender = InterceptedSender<<Context as overseer::SubsystemContext>::Sender, Fil>;
	type Error = SubsystemError;
	type OutgoingMessages = <<Context as overseer::SubsystemContext>::Message as overseer::AssociateOutgoing>::OutgoingMessages;
	type Signal = OverseerSignal;

	async fn try_recv(&mut self) -> Result<Option<FromOrchestra<Self::Message>>, ()> {
		loop {
			match self.inner.try_recv().await? {
				None => return Ok(None),
				Some(msg) =>
					if let Some(msg) =
						self.message_filter.intercept_incoming(self.inner.sender(), msg)
					{
						return Ok(Some(msg))
					},
			}
		}
	}

	async fn recv(&mut self) -> SubsystemResult<FromOrchestra<Self::Message>> {
		if let Some(msg) = self.message_buffer.pop_front() {
			return Ok(msg)
		}
		loop {
			let msg = self.inner.recv().await?;
			if let Some(msg) = self.message_filter.intercept_incoming(self.inner.sender(), msg) {
				return Ok(msg)
			}
		}
	}

	async fn recv_signal(&mut self) -> SubsystemResult<Self::Signal> {
		loop {
			let msg = self.inner.recv().await?;
			if let Some(msg) = self.message_filter.intercept_incoming(self.inner.sender(), msg) {
				if let FromOrchestra::Signal(sig) = msg {
					return Ok(sig)
				} else {
					self.message_buffer.push_back(msg)
				}
			}
		}
	}

	fn spawn(
		&mut self,
		name: &'static str,
		s: Pin<Box<dyn Future<Output = ()> + Send>>,
	) -> SubsystemResult<()> {
		self.inner.spawn(name, s)
	}

	fn spawn_blocking(
		&mut self,
		name: &'static str,
		s: Pin<Box<dyn Future<Output = ()> + Send>>,
	) -> SubsystemResult<()> {
		self.inner.spawn_blocking(name, s)
	}

	fn sender(&mut self) -> &mut Self::Sender {
		&mut self.sender
	}
}

/// A subsystem to which incoming and outgoing filters are applied.
pub struct InterceptedSubsystem<Sub, Interceptor> {
	pub subsystem: Sub,
	pub message_interceptor: Interceptor,
}

impl<Sub, Interceptor> InterceptedSubsystem<Sub, Interceptor> {
	pub fn new(subsystem: Sub, message_interceptor: Interceptor) -> Self {
		Self { subsystem, message_interceptor }
	}
}

impl<Context, Sub, Interceptor> overseer::Subsystem<Context, SubsystemError> for InterceptedSubsystem<Sub, Interceptor>
where
	Context:
		overseer::SubsystemContext<Error=SubsystemError,Signal=OverseerSignal> + Sync + Send,
	InterceptedContext<Context, Interceptor>:
		overseer::SubsystemContext<Error=SubsystemError,Signal=OverseerSignal>,
	Sub:
		overseer::Subsystem<InterceptedContext<Context, Interceptor>, SubsystemError>,
	Interceptor:
		MessageInterceptor<
			<Context as overseer::SubsystemContext>::Sender,
			Message = <Context as overseer::SubsystemContext>::Message,
		>,
	<Context as overseer::SubsystemContext>::Message:
		overseer::AssociateOutgoing,
	<Context as overseer::SubsystemContext>::Sender:
		overseer::SubsystemSender<
				<<Context as overseer::SubsystemContext>::Message as overseer::AssociateOutgoing
			>::OutgoingMessages
		>,
{
	fn start(self, ctx: Context) -> SpawnedSubsystem {
		let ctx = InterceptedContext::new(ctx, self.message_interceptor);
		overseer::Subsystem::<InterceptedContext<Context, Interceptor>, SubsystemError>::start(
			self.subsystem,
			ctx,
		)
	}
}
