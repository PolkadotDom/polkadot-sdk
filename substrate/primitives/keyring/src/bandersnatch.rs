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

//! A set of well-known keys used for testing.

pub use sp_core::bandersnatch;

use crate::ParseKeyringError;
#[cfg(feature = "std")]
use sp_core::bandersnatch::Signature;
use sp_core::{
	bandersnatch::{Pair, Public},
	crypto::UncheckedFrom,
	hex2array, ByteArray, Pair as PairT,
};

extern crate alloc;
use alloc::{format, str::FromStr, string::String, vec::Vec};

/// Set of test accounts.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumIter, Ord, PartialOrd,
)]
pub enum Keyring {
	Alice,
	Bob,
	Charlie,
	Dave,
	Eve,
	Ferdie,
	One,
	Two,
}

const PUBLIC_RAW_LEN: usize = <Public as ByteArray>::LEN;

impl Keyring {
	pub fn from_public(who: &Public) -> Option<Keyring> {
		Self::iter().find(|&k| &Public::from(k) == who)
	}

	pub fn from_raw_public(who: [u8; PUBLIC_RAW_LEN]) -> Option<Keyring> {
		Self::from_public(&Public::unchecked_from(who))
	}

	pub fn to_raw_public(self) -> [u8; PUBLIC_RAW_LEN] {
		*Public::from(self).as_ref()
	}

	pub fn to_raw_public_vec(self) -> Vec<u8> {
		Public::from(self).to_raw_vec()
	}

	#[cfg(feature = "std")]
	pub fn sign(self, msg: &[u8]) -> Signature {
		Pair::from(self).sign(msg)
	}

	pub fn pair(self) -> Pair {
		Pair::from_string(&format!("//{}", <&'static str>::from(self)), None)
			.expect("static values are known good; qed")
	}

	/// Returns an iterator over all test accounts.
	pub fn iter() -> impl Iterator<Item = Keyring> {
		<Self as strum::IntoEnumIterator>::iter()
	}

	pub fn public(self) -> Public {
		Public::from(self)
	}

	pub fn to_seed(self) -> String {
		format!("//{}", self)
	}

	/// Create a crypto `Pair` from a numeric value.
	pub fn numeric(idx: usize) -> Pair {
		Pair::from_string(&format!("//{}", idx), None).expect("numeric values are known good; qed")
	}
}

impl From<Keyring> for &'static str {
	fn from(k: Keyring) -> Self {
		match k {
			Keyring::Alice => "Alice",
			Keyring::Bob => "Bob",
			Keyring::Charlie => "Charlie",
			Keyring::Dave => "Dave",
			Keyring::Eve => "Eve",
			Keyring::Ferdie => "Ferdie",
			Keyring::One => "One",
			Keyring::Two => "Two",
		}
	}
}

impl FromStr for Keyring {
	type Err = ParseKeyringError;

	fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
		match s {
			"Alice" => Ok(Keyring::Alice),
			"Bob" => Ok(Keyring::Bob),
			"Charlie" => Ok(Keyring::Charlie),
			"Dave" => Ok(Keyring::Dave),
			"Eve" => Ok(Keyring::Eve),
			"Ferdie" => Ok(Keyring::Ferdie),
			"One" => Ok(Keyring::One),
			"Two" => Ok(Keyring::Two),
			_ => Err(ParseKeyringError),
		}
	}
}

impl From<Keyring> for Public {
	fn from(k: Keyring) -> Self {
		Public::unchecked_from(<[u8; PUBLIC_RAW_LEN]>::from(k))
	}
}

impl From<Keyring> for Pair {
	fn from(k: Keyring) -> Self {
		k.pair()
	}
}

impl From<Keyring> for [u8; PUBLIC_RAW_LEN] {
	fn from(k: Keyring) -> Self {
		match k {
			Keyring::Alice =>
				hex2array!("4d8e57b723e8bb4eca5c28d79cb95b9e84b4e2319d9851d45504014633e55d01"),
			Keyring::Bob =>
				hex2array!("aa6daf4784d581804d8f5cc1edb2ad171dbdf9c5188ddc071b11c3479c150c37"),
			Keyring::Charlie =>
				hex2array!("331d681392223b35b92319e059d3dbc2869b2ef74400a70e678b4a5108c81ec0"),
			Keyring::Dave =>
				hex2array!("374384c19a877040c84bb07fcf3aac74ff7fafface0b1c01a93fd2ddbf5c1660"),
			Keyring::Eve =>
				hex2array!("14bdd9381e80c07b75b8f1e92d6b2e4652e5135beaad1eedb1b81ff01ee562ad"),
			Keyring::Ferdie =>
				hex2array!("e13bd31b0575076479914c16c5ad69779f206375dbf19519219eeba3b10cc063"),
			Keyring::One =>
				hex2array!("03466a4de97ae18bc4604a3c40dfbddc6bac9f707c9b3c31a94a2c1725a03253"),
			Keyring::Two =>
				hex2array!("0fda0d1336e8d6ee687ebf6d14eaa9b92b5601068e6159222623c8e14c004293"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_core::{bandersnatch::Pair, Pair as PairT};

	#[test]
	fn should_work() {
		assert!(Pair::verify(
			&Keyring::Alice.sign(b"I am Alice!"),
			b"I am Alice!",
			&Keyring::Alice.public(),
		));
		assert!(!Pair::verify(
			&Keyring::Alice.sign(b"I am Alice!"),
			b"I am Bob!",
			&Keyring::Alice.public(),
		));
		assert!(!Pair::verify(
			&Keyring::Alice.sign(b"I am Alice!"),
			b"I am Alice!",
			&Keyring::Bob.public(),
		));
	}
	#[test]
	fn verify_static_public_keys() {
		assert!(Keyring::iter()
			.all(|k| { k.pair().public().as_ref() == <[u8; PUBLIC_RAW_LEN]>::from(k) }));
	}
}
