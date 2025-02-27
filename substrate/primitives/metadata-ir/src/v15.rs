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

//! Convert the IR to V15 metadata.

use super::types::{
	ExtrinsicMetadataIR, MetadataIR, OuterEnumsIR, PalletMetadataIR, RuntimeApiMetadataIR,
	RuntimeApiMethodMetadataIR, RuntimeApiMethodParamMetadataIR, TransactionExtensionMetadataIR,
};

use frame_metadata::v15::{
	CustomMetadata, ExtrinsicMetadata, OuterEnums, PalletMetadata, RuntimeApiMetadata,
	RuntimeApiMethodMetadata, RuntimeApiMethodParamMetadata, RuntimeMetadataV15,
	SignedExtensionMetadata,
};
use scale_info::{IntoPortable, Registry};

impl From<MetadataIR> for RuntimeMetadataV15 {
	fn from(ir: MetadataIR) -> Self {
		let mut registry = Registry::new();
		let pallets =
			registry.map_into_portable(ir.pallets.into_iter().map(Into::<PalletMetadata>::into));
		let extrinsic = Into::<ExtrinsicMetadata>::into(ir.extrinsic).into_portable(&mut registry);
		let ty = registry.register_type(&ir.ty);
		let apis =
			registry.map_into_portable(ir.apis.into_iter().map(Into::<RuntimeApiMetadata>::into));
		let outer_enums = Into::<OuterEnums>::into(ir.outer_enums).into_portable(&mut registry);
		let custom = CustomMetadata { map: Default::default() };

		Self { types: registry.into(), pallets, extrinsic, ty, apis, outer_enums, custom }
	}
}

impl From<RuntimeApiMetadataIR> for RuntimeApiMetadata {
	fn from(ir: RuntimeApiMetadataIR) -> Self {
		RuntimeApiMetadata {
			name: ir.name,
			methods: ir.methods.into_iter().map(Into::into).collect(),
			docs: ir.docs,
		}
	}
}

impl From<RuntimeApiMethodMetadataIR> for RuntimeApiMethodMetadata {
	fn from(ir: RuntimeApiMethodMetadataIR) -> Self {
		RuntimeApiMethodMetadata {
			name: ir.name,
			inputs: ir.inputs.into_iter().map(Into::into).collect(),
			output: ir.output,
			docs: ir.docs,
		}
	}
}

impl From<RuntimeApiMethodParamMetadataIR> for RuntimeApiMethodParamMetadata {
	fn from(ir: RuntimeApiMethodParamMetadataIR) -> Self {
		RuntimeApiMethodParamMetadata { name: ir.name, ty: ir.ty }
	}
}

impl From<PalletMetadataIR> for PalletMetadata {
	fn from(ir: PalletMetadataIR) -> Self {
		PalletMetadata {
			name: ir.name,
			storage: ir.storage.map(Into::into),
			calls: ir.calls.map(Into::into),
			event: ir.event.map(Into::into),
			constants: ir.constants.into_iter().map(Into::into).collect(),
			error: ir.error.map(Into::into),
			index: ir.index,
			docs: ir.docs,
		}
	}
}

impl From<TransactionExtensionMetadataIR> for SignedExtensionMetadata {
	fn from(ir: TransactionExtensionMetadataIR) -> Self {
		SignedExtensionMetadata {
			identifier: ir.identifier,
			ty: ir.ty,
			additional_signed: ir.implicit,
		}
	}
}

impl From<ExtrinsicMetadataIR> for ExtrinsicMetadata {
	fn from(ir: ExtrinsicMetadataIR) -> Self {
		ExtrinsicMetadata {
			version: *ir.versions.iter().min().expect("Metadata V15 supports only one version"),
			address_ty: ir.address_ty,
			call_ty: ir.call_ty,
			signature_ty: ir.signature_ty,
			extra_ty: ir.extra_ty,
			signed_extensions: ir.extensions.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<OuterEnumsIR> for OuterEnums {
	fn from(ir: OuterEnumsIR) -> Self {
		OuterEnums {
			call_enum_ty: ir.call_enum_ty,
			event_enum_ty: ir.event_enum_ty,
			error_enum_ty: ir.error_enum_ty,
		}
	}
}
