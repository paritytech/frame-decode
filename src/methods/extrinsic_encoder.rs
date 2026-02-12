// Copyright (C) 2022-2026 Parity Technologies (UK) Ltd. (admin@parity.io)
// This file is a part of the frame-decode crate.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod transaction_extension;
mod transaction_extensions;
use super::extrinsic_type_info::{
    ExtrinsicCallInfo, ExtrinsicExtensionInfo, ExtrinsicInfoError, ExtrinsicSignatureInfo,
    ExtrinsicTypeInfo,
};
use alloc::vec::Vec;
use parity_scale_codec::Encode;
use scale_encode::{EncodeAsFields, EncodeAsType};
use scale_type_resolver::{Field, TypeResolver};

pub use transaction_extension::{TransactionExtension, TransactionExtensionError};
pub use transaction_extensions::{TransactionExtensions, TransactionExtensionsError};

/// An error returned trying to encode extrinsic call data.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum ExtrinsicEncodeError {
    #[error("Cannot get extrinsic info: {0}")]
    CannotGetInfo(ExtrinsicInfoError<'static>),
    #[error("Extrinsic encoding failed: cannot encode call data: {0}")]
    CannotEncodeCallData(scale_encode::Error),
    #[error("Extrinsic encoding failed: cannot encode address: {0}")]
    CannotEncodeAddress(scale_encode::Error),
    #[error("Extrinsic encoding failed: cannot encode signature: {0}")]
    CannotEncodeSignature(scale_encode::Error),
    #[error("Extrinsic encoding failed: cannot encode transaction extensions: {0}")]
    TransactionExtensions(TransactionExtensionsError),
    #[error(
        "Extrinsic encoding failed: cannot find a transaction extension version which enough data was given for."
    )]
    CannotFindGoodExtensionVersion,
}

/// Encode a V4 unsigned extrinsic (also known as an inherent).
///
/// This is the same as [`encode_v4_unsigned_to`], but returns the encoded extrinsic as a `Vec<u8>`,
/// rather than accepting a mutable output buffer.
///
/// # Example
///
/// ```rust
/// use frame_decode::extrinsics::encode_v4_unsigned;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// // Encode a call to Timestamp.set with an argument.
/// // The call_data type must implement `scale_encode::EncodeAsFields`.
/// let call_data = scale_value::value!({
///     now: 1234567890u64,
/// });
///
/// let encoded = encode_v4_unsigned(
///     "Timestamp",
///     "set",
///     &call_data,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
pub fn encode_v4_unsigned<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let mut out = Vec::new();
    encode_v4_unsigned_to(
        pallet_name,
        call_name,
        call_data,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode a V4 unsigned extrinsic (also known as an inherent) to a provided output buffer.
///
/// This is the same as [`encode_v4_unsigned`], but writes the encoded extrinsic to the provided
/// `Vec<u8>` rather than returning a new one.
///
/// # Example
///
/// ```rust
/// use frame_decode::extrinsics::encode_v4_unsigned_to;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// // Encode a call to Timestamp.set with an argument.
/// let call_data = scale_value::value!({
///     now: 1234567890u64,
/// });
///
/// let mut encoded = Vec::new();
/// encode_v4_unsigned_to(
///     "Timestamp",
///     "set",
///     &call_data,
///     &metadata,
///     &metadata.types,
///     &mut encoded,
/// ).unwrap();
/// ```
pub fn encode_v4_unsigned_to<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v4_unsigned_with_info_to(call_data, type_resolver, &call_info, out)
}

/// Encode a V4 unsigned extrinsic (also known as an inherent) to a provided output buffer,
/// using pre-computed call information.
///
/// Unlike [`encode_v4_unsigned_to`], which obtains the call info internally given the pallet and call names,
/// this function takes the call info as an argument. This is useful if you already have the call info available,
/// for example if you are encoding multiple extrinsics for the same call.
pub fn encode_v4_unsigned_with_info_to<CallData, Resolver>(
    call_data: &CallData,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
{
    encode_unsigned_at_version_with_info_to(
        call_data,
        call_info,
        type_resolver,
        TransactionVersion::V4,
        out,
    )
}

/// Encode a V4 signed extrinsic, ready to submit.
///
/// A signed V4 extrinsic includes an address, signature, and transaction extensions (such as
/// nonce and tip) alongside the call data. The signature should be computed over the signer
/// payload, which can be obtained via [`encode_v4_signer_payload`].
///
/// This is the same as [`encode_v4_signed_to`], but returns the encoded extrinsic as a `Vec<u8>`,
/// rather than accepting a mutable output buffer.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v4_signed, TransactionExtensions};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// // The call data, address, signature, and transaction extensions must implement
/// // the appropriate scale_encode traits.
/// let call_data = /* ... */;
/// let address = /* your address type */;
/// let signature = /* your signature type */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// let encoded = encode_v4_signed(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     &transaction_extensions,
///     &address,
///     &signature,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
#[allow(clippy::too_many_arguments)]
pub fn encode_v4_signed<CallData, Info, Resolver, Exts, Address, Signature>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extensions: &Exts,
    address: &Address,
    signature: &Signature,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
    Address: EncodeAsType,
    Signature: EncodeAsType,
{
    let mut out = Vec::new();
    encode_v4_signed_to(
        pallet_name,
        call_name,
        call_data,
        transaction_extensions,
        address,
        signature,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode a V4 signed extrinsic to a provided output buffer.
///
/// A signed extrinsic includes an address, signature, and transaction extensions (such as
/// nonce and tip) alongside the call data. The signature should be computed over the signer
/// payload, which can be obtained via [`encode_v4_signer_payload`].
///
/// This is the same as [`encode_v4_signed`], but writes the encoded extrinsic to the provided
/// `Vec<u8>` rather than returning a new one.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v4_signed_to, TransactionExtensions};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let call_data = /* ... */;
/// let address = /* your address type */;
/// let signature = /* your signature type */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// let mut encoded = Vec::new();
/// encode_v4_signed_to(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     &transaction_extensions,
///     &address,
///     &signature,
///     &metadata,
///     &metadata.types,
///     &mut encoded,
/// ).unwrap();
/// ```
#[allow(clippy::too_many_arguments)]
pub fn encode_v4_signed_to<CallData, Info, Resolver, Exts, Address, Signature>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extensions: &Exts,
    address: &Address,
    signature: &Signature,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
    Address: EncodeAsType,
    Signature: EncodeAsType,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    let ext_info = info
        .extrinsic_extension_info(None)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    let sig_info = info
        .extrinsic_signature_info()
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v4_signed_with_info_to(
        call_data,
        transaction_extensions,
        address,
        signature,
        type_resolver,
        &call_info,
        &sig_info,
        &ext_info,
        out,
    )
}

/// Encode a V4 signed extrinsic to a provided output buffer, using pre-computed type information.
///
/// Unlike [`encode_v4_signed_to`], which obtains the call, signature, and extension info internally
/// given the pallet and call names, this function takes these as arguments. This is useful if you
/// already have this information available, for example if you are encoding multiple extrinsics.
#[allow(clippy::too_many_arguments)]
pub fn encode_v4_signed_with_info_to<CallData, Resolver, Exts, Address, Signature>(
    call_data: &CallData,
    transaction_extensions: &Exts,
    address: &Address,
    signature: &Signature,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    sig_info: &ExtrinsicSignatureInfo<Resolver::TypeId>,
    ext_info: &ExtrinsicExtensionInfo<Resolver::TypeId>,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
    Exts: TransactionExtensions<Resolver>,
    Address: EncodeAsType,
    Signature: EncodeAsType,
{
    // Encode the "inner" bytes
    let mut encoded_inner = Vec::new();

    // "is signed" + transaction protocol version (4)
    (0b10000000 + 4u8).encode_to(&mut encoded_inner);

    // Who is this transaction from (corresponds to public key of signature)
    address
        .encode_as_type_to(
            sig_info.address_id.clone(),
            type_resolver,
            &mut encoded_inner,
        )
        .map_err(ExtrinsicEncodeError::CannotEncodeAddress)?;

    // Signature for the above identity
    signature
        .encode_as_type_to(
            sig_info.signature_id.clone(),
            type_resolver,
            &mut encoded_inner,
        )
        .map_err(ExtrinsicEncodeError::CannotEncodeSignature)?;

    // Signed extensions (now Transaction Extensions)
    for (name, id) in iter_nonempty_extention_values(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_value_to(name, id, type_resolver, &mut encoded_inner)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // And now the actual call data, ie the arguments we're passing to the call
    encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut encoded_inner)?;

    // Now, encoding these inner bytes prefixes the compact length to the beginning:
    encoded_inner.encode_to(out);
    Ok(())
}

/// Encode the signer payload for a V4 signed extrinsic.
///
/// The signer payload is the data that should be signed to produce the signature for
/// a signed extrinsic. It consists of the encoded call data, the transaction extension
/// values, and the transaction extension implicit data. If the resulting payload exceeds
/// 256 bytes, it is hashed using Blake2-256.
///
/// Use this function to obtain the bytes that should be signed, then pass the resulting
/// signature to [`encode_v4_signed`] to construct the final extrinsic.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v4_signer_payload, TransactionExtensions};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let call_data = /* ... */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// // Get the payload to sign
/// let signer_payload = encode_v4_signer_payload(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     &transaction_extensions,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
///
/// // Sign the payload with your signing key, then use encode_v4_signed
/// // to construct the final extrinsic.
/// ```
pub fn encode_v4_signer_payload<CallData, Info, Resolver, Exts>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extensions: &Exts,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
    Info::TypeId: Clone,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    let ext_info = info
        .extrinsic_extension_info(None)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v4_signer_payload_with_info(
        call_data,
        transaction_extensions,
        type_resolver,
        &call_info,
        &ext_info,
    )
}

/// Encode the signer payload for a V4 signed extrinsic, using pre-computed type information.
///
/// Unlike [`encode_v4_signer_payload`], which obtains the call and extension info internally
/// given the pallet and call names, this function takes these as arguments. This is useful if you
/// already have this information available.
///
/// The signer payload consists of the encoded call data, the transaction extension values,
/// and the transaction extension implicit data. If the resulting payload exceeds 256 bytes,
/// it is hashed using Blake2-256.
pub fn encode_v4_signer_payload_with_info<CallData, Resolver, Exts>(
    call_data: &CallData,
    transaction_extensions: &Exts,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    ext_info: &ExtrinsicExtensionInfo<Resolver::TypeId>,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
    Exts: TransactionExtensions<Resolver>,
{
    let mut out = Vec::new();

    // First encode call data
    encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut out)?;

    // Then the signer payload value (ie roughly the bytes that will appear in the tx)
    for (name, id) in iter_nonempty_extention_values(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_value_for_signer_payload_to(name, id, type_resolver, &mut out)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // Then the signer payload implicits (ie data we want to verify that is NOT in the tx)
    for (name, id) in iter_nonempty_extention_implicits(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_implicit_to(name, id, type_resolver, &mut out)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // Finally we need to hash it if it's too long
    if out.len() > 256 {
        out = sp_crypto_hashing::blake2_256(&out).to_vec();
    }

    Ok(out)
}

/// Encode a V5 bare extrinsic (also known as an inherent), ready to submit.
///
/// V5 bare extrinsics contain only call data with no transaction extensions or signature.
/// They are functionally equivalent to V4 unsigned extrinsics and are typically used for
/// inherents (data provided by block authors).
///
/// This is the same as [`encode_v5_bare_to`], but returns the encoded extrinsic as a `Vec<u8>`,
/// rather than accepting a mutable output buffer.
///
/// # Example
///
/// ```rust
/// use frame_decode::extrinsics::encode_v5_bare;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// // Encode a call to Timestamp.set with an argument.
/// let call_data = scale_value::value!({
///     now: 1234567890u64,
/// });
///
/// let encoded = encode_v5_bare(
///     "Timestamp",
///     "set",
///     &call_data,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
pub fn encode_v5_bare<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let mut out = Vec::new();
    encode_v5_bare_to(
        pallet_name,
        call_name,
        call_data,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode a V5 bare extrinsic (also known as an inherent) to a provided output buffer.
///
/// This is the same as [`encode_v5_bare`], but writes the encoded extrinsic to the provided
/// `Vec<u8>` rather than returning a new one.
///
/// # Example
///
/// ```rust
/// use frame_decode::extrinsics::encode_v5_bare_to;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// // Encode a call to Timestamp.set with an argument.
/// let call_data = scale_value::value!({
///     now: 1234567890u64,
/// });
///
/// let mut encoded = Vec::new();
/// encode_v5_bare_to(
///     "Timestamp",
///     "set",
///     &call_data,
///     &metadata,
///     &metadata.types,
///     &mut encoded,
/// ).unwrap();
/// ```
pub fn encode_v5_bare_to<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v5_bare_with_info_to(call_data, type_resolver, &call_info, out)
}

/// Encode a V5 bare extrinsic (also known as an inherent) to a provided output buffer,
/// using pre-computed call information.
///
/// Unlike [`encode_v5_bare_to`], which obtains the call info internally given the pallet and call names,
/// this function takes the call info as an argument. This is useful if you already have the call info available,
/// for example if you are encoding multiple extrinsics for the same call.
pub fn encode_v5_bare_with_info_to<CallData, Resolver>(
    call_data: &CallData,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
{
    encode_unsigned_at_version_with_info_to(
        call_data,
        call_info,
        type_resolver,
        TransactionVersion::V5,
        out,
    )
}

/// Determine the best transaction extension version to use for a V5 general extrinsic.
///
/// V5 general extrinsics support multiple versions of transaction extensions. This function
/// iterates through the available extension versions and returns the first version for which
/// all required extension data is provided.
///
/// Use this function to determine which `transaction_extension_version` to pass to
/// [`encode_v5_general`] or [`encode_v5_general_to`].
///
/// # Errors
///
/// Returns [`ExtrinsicEncodeError::CannotFindGoodExtensionVersion`] if no extension version
/// can be found for which all required data is available.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{best_v5_general_transaction_extension_version, encode_v5_general};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata.scale").unwrap();
/// let RuntimeMetadata::V16(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// // Find the best extension version for your provided extensions
/// let ext_version = best_v5_general_transaction_extension_version(
///     &transaction_extensions,
///     &metadata,
/// ).unwrap();
///
/// // Use this version when encoding the extrinsic
/// let encoded = encode_v5_general(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     ext_version,
///     &transaction_extensions,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
pub fn best_v5_general_transaction_extension_version<Exts, Info, Resolver>(
    transaction_extensions: &Exts,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<u8, ExtrinsicEncodeError>
where
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info::TypeId: Clone,
{
    let extension_versions = info
        .extrinsic_extension_version_info()
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    for ext_version in extension_versions {
        // get extension info for each version.
        let ext_info = info
            .extrinsic_extension_info(Some(ext_version))
            .map_err(|i| i.into_owned())
            .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

        // Do we have all of the extension data for this version?
        let have_data = ext_info.extension_ids.iter().all(|e| {
            let is_value_empty = is_type_empty(e.id.clone(), type_resolver);
            let is_implicit_empty = is_type_empty(e.implicit_id.clone(), type_resolver);
            (is_value_empty && is_implicit_empty)
                || transaction_extensions.contains_extension(&e.name)
        });

        // If we have all of the data we need, encode the extrinsic,
        // else loop and try the next extension version.
        if have_data {
            return Ok(ext_version);
        }
    }

    Err(ExtrinsicEncodeError::CannotFindGoodExtensionVersion)
}

/// Encode a V5 general extrinsic, ready to submit.
///
/// V5 general extrinsics include transaction extensions but no separate signature field.
/// Instead, the signature (if needed) is provided as part of one of the transaction extensions.
/// This is the new extrinsic format introduced in newer Substrate runtimes.
///
/// Use [`best_v5_general_transaction_extension_version`] to determine which extension version
/// to use based on the extensions you have available.
///
/// This is the same as [`encode_v5_general_to`], but returns the encoded extrinsic as a `Vec<u8>`,
/// rather than accepting a mutable output buffer.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v5_general, best_v5_general_transaction_extension_version};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata.scale").unwrap();
/// let RuntimeMetadata::V16(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let call_data = /* ... */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// let ext_version = best_v5_general_transaction_extension_version(
///     &transaction_extensions,
///     &metadata,
/// ).unwrap();
///
/// let encoded = encode_v5_general(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     ext_version,
///     &transaction_extensions,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
pub fn encode_v5_general<CallData, Info, Resolver, Exts>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extension_version: u8,
    transaction_extensions: &Exts,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
{
    let mut out = Vec::new();
    encode_v5_general_to(
        pallet_name,
        call_name,
        call_data,
        transaction_extension_version,
        transaction_extensions,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode a V5 general extrinsic to a provided output buffer.
///
/// V5 general extrinsics include transaction extensions but no separate signature field.
/// Instead, the signature (if needed) is provided as part of one of the transaction extensions.
///
/// This is the same as [`encode_v5_general`], but writes the encoded extrinsic to the provided
/// `Vec<u8>` rather than returning a new one.
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v5_general_to, best_v5_general_transaction_extension_version};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata.scale").unwrap();
/// let RuntimeMetadata::V16(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let call_data = /* ... */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// let ext_version = best_v5_general_transaction_extension_version(
///     &transaction_extensions,
///     &metadata,
/// ).unwrap();
///
/// let mut encoded = Vec::new();
/// encode_v5_general_to(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     ext_version,
///     &transaction_extensions,
///     &metadata,
///     &metadata.types,
///     &mut encoded,
/// ).unwrap();
/// ```
#[allow(clippy::too_many_arguments)]
pub fn encode_v5_general_to<CallData, Info, Resolver, Exts>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extension_version: u8,
    transaction_extensions: &Exts,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    let ext_info = info
        .extrinsic_extension_info(Some(transaction_extension_version))
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v5_general_with_info_to(
        call_data,
        transaction_extension_version,
        transaction_extensions,
        type_resolver,
        &call_info,
        &ext_info,
        out,
    )
}

/// Encode a V5 general extrinsic to a provided output buffer, using pre-computed type information.
///
/// Unlike [`encode_v5_general_to`], which obtains the call and extension info internally
/// given the pallet and call names, this function takes these as arguments. This is useful if you
/// already have this information available, for example if you are encoding multiple extrinsics.
pub fn encode_v5_general_with_info_to<CallData, Resolver, Exts>(
    call_data: &CallData,
    transaction_extension_version: u8,
    transaction_extensions: &Exts,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    ext_info: &ExtrinsicExtensionInfo<Resolver::TypeId>,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
    Exts: TransactionExtensions<Resolver>,
{
    // Encode the "inner" bytes
    let mut encoded_inner = Vec::new();

    // "is signed" + transaction protocol version (4)
    (0b01000000 + 5u8).encode_to(&mut encoded_inner);

    // Version of the transaction extensions.
    transaction_extension_version.encode_to(&mut encoded_inner);

    // Transaction Extensions next. These may include a signature/address
    for (name, id) in iter_nonempty_extention_values(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_value_to(name, id, type_resolver, &mut encoded_inner)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // And now the actual call data, ie the arguments we're passing to the call
    encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut encoded_inner)?;

    // Now, encoding these inner bytes prefixes the compact length to the beginning:
    encoded_inner.encode_to(out);
    Ok(())
}

/// Encode the signer payload for a V5 general extrinsic.
///
/// The signer payload is the data that should be signed to produce the signature for
/// a general extrinsic. It consists of the encoded call data, the transaction extension
/// values (for the signer payload), and the transaction extension implicit data.
///
/// Unlike [`encode_v4_signer_payload`], which conditionally hashes the payload if it exceeds
/// 256 bytes, V5 signer payloads are always hashed using Blake2-256, returning a fixed 32-byte
/// array.
///
/// Use this function to obtain the bytes that should be signed, then include the resulting
/// signature in the appropriate transaction extension when calling [`encode_v5_general`].
///
/// # Example
///
/// ```rust,ignore
/// use frame_decode::extrinsics::{encode_v5_signer_payload, best_v5_general_transaction_extension_version};
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata.scale").unwrap();
/// let RuntimeMetadata::V16(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let call_data = /* ... */;
/// let transaction_extensions = /* your TransactionExtensions impl */;
///
/// let ext_version = best_v5_general_transaction_extension_version(
///     &transaction_extensions,
///     &metadata,
/// ).unwrap();
///
/// // Get the 32-byte payload hash to sign
/// let signer_payload = encode_v5_signer_payload(
///     "Balances",
///     "transfer_keep_alive",
///     &call_data,
///     ext_version,
///     &transaction_extensions,
///     &metadata,
///     &metadata.types,
/// ).unwrap();
///
/// // Sign the payload with your signing key, then include the signature
/// // in your transaction extensions when calling encode_v5_general.
/// ```
pub fn encode_v5_signer_payload<CallData, Info, Resolver, Exts>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    transaction_extension_version: u8,
    transaction_extensions: &Exts,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<[u8; 32], ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
    Exts: TransactionExtensions<Resolver>,
    Info::TypeId: Clone,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    let ext_info = info
        .extrinsic_extension_info(Some(transaction_extension_version))
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_v5_signer_payload_with_info(
        call_data,
        transaction_extensions,
        type_resolver,
        &call_info,
        &ext_info,
    )
}

/// Encode the signer payload for a V5 general extrinsic, using pre-computed type information.
///
/// Unlike [`encode_v5_signer_payload`], which obtains the call and extension info internally
/// given the pallet and call names, this function takes these as arguments. This is useful if you
/// already have this information available.
///
/// The signer payload consists of the encoded call data, the transaction extension values
/// (for the signer payload), and the transaction extension implicit data. The result is always
/// hashed using Blake2-256, returning a fixed 32-byte array.
pub fn encode_v5_signer_payload_with_info<CallData, Resolver, Exts>(
    call_data: &CallData,
    transaction_extensions: &Exts,
    type_resolver: &Resolver,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    ext_info: &ExtrinsicExtensionInfo<Resolver::TypeId>,
) -> Result<[u8; 32], ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver,
    Exts: TransactionExtensions<Resolver>,
{
    let mut out = Vec::new();

    // First encode call data
    encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut out)?;

    // Then the signer payload value (ie roughly the bytes that will appear in the tx)
    for (name, id) in iter_nonempty_extention_values(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_value_for_signer_payload_to(name, id, type_resolver, &mut out)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // Then the signer payload implicits (ie data we want to verify that is NOT in the tx)
    for (name, id) in iter_nonempty_extention_implicits(ext_info, type_resolver) {
        transaction_extensions
            .encode_extension_implicit_to(name, id, type_resolver, &mut out)
            .map_err(ExtrinsicEncodeError::TransactionExtensions)?;
    }

    // Finally hash it (regardless of length).
    Ok(sp_crypto_hashing::blake2_256(&out))
}

/// Encode the call data for an extrinsic.
///
/// This is basically an alias for [`scale_encode::EncodeAsFields::encode_as_fields()`].
pub fn encode_call_data<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let mut out = Vec::new();
    encode_call_data_to(
        pallet_name,
        call_name,
        call_data,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode the call data for an extrinsic to the given Vec.
///
/// This is basically an alias for [`scale_encode::EncodeAsFields::encode_as_fields()`], but
/// with a byte for the pallet index and call index prepended.
pub fn encode_call_data_to<CallData, Info, Resolver>(
    pallet_name: &str,
    call_name: &str,
    call_data: &CallData,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    CallData: EncodeAsFields,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
    Info: ExtrinsicTypeInfo,
{
    let call_info = info
        .extrinsic_call_info_by_name(pallet_name, call_name)
        .map_err(|i| i.into_owned())
        .map_err(ExtrinsicEncodeError::CannotGetInfo)?;

    encode_call_data_with_info_to(call_data, &call_info, type_resolver, out)
}

/// Encode the call data for an extrinsic, given some already-computed [`ExtrinsicCallInfo`].
///
/// This is basically an alias for [`scale_encode::EncodeAsFields::encode_as_fields()`], but
/// with a byte for the pallet index and call index prepended.
pub fn encode_call_data_with_info<CallData, Info, Resolver>(
    call_data: &CallData,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, ExtrinsicEncodeError>
where
    Resolver: TypeResolver,
    CallData: EncodeAsFields,
{
    let mut out = Vec::new();
    encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut out)?;
    Ok(out)
}

/// Encode the call data for an extrinsic, given some already-computed [`ExtrinsicCallInfo`],
/// to the given Vec.
///
/// This is basically an alias for [`scale_encode::EncodeAsFields::encode_as_fields()`], but
/// with a byte for the pallet index and call index prepended.
pub fn encode_call_data_with_info_to<CallData, Resolver>(
    call_data: &CallData,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    type_resolver: &Resolver,
    mut out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    Resolver: TypeResolver,
    CallData: EncodeAsFields,
{
    // Pallet and call index to identify the call:
    call_info.pallet_index.encode_to(&mut out);
    call_info.call_index.encode_to(&mut out);

    // Arguments to this call:
    let mut fields = call_info.args.iter().map(|arg| Field {
        name: Some(&*arg.name),
        id: arg.id.clone(),
    });
    call_data
        .encode_as_fields_to(&mut fields, type_resolver, out)
        .map_err(ExtrinsicEncodeError::CannotEncodeCallData)?;

    Ok(())
}

// V4 unsigned and V5 bare extrinsics are basically encoded
// in the same way; this helper can do either.
fn encode_unsigned_at_version_with_info_to<CallData, Resolver>(
    call_data: &CallData,
    call_info: &ExtrinsicCallInfo<Resolver::TypeId>,
    type_resolver: &Resolver,
    tx_version: TransactionVersion,
    out: &mut Vec<u8>,
) -> Result<(), ExtrinsicEncodeError>
where
    Resolver: TypeResolver,
    CallData: EncodeAsFields,
{
    // Build our inner, non-length-prefixed extrinsic:
    let inner = {
        let mut out = Vec::new();
        // Transaction version (4):
        (tx_version as u8).encode_to(&mut out);
        // Then the arguments for the call:
        encode_call_data_with_info_to(call_data, call_info, type_resolver, &mut out)?;
        out
    };

    // Encode the inner vec to prefix the compact length to it:
    inner.encode_to(out);
    Ok(())
}

#[derive(Copy, Clone)]
#[repr(u8)]
enum TransactionVersion {
    V4 = 4u8,
    V5 = 5u8,
}

/// Iterate over the non-empty extension implicit name/IDs
fn iter_nonempty_extention_implicits<'exts, 'info, Resolver: TypeResolver>(
    extension_info: &'exts ExtrinsicExtensionInfo<'info, Resolver::TypeId>,
    types: &Resolver,
) -> impl Iterator<Item = (&'exts str, Resolver::TypeId)> {
    extension_info
        .extension_ids
        .iter()
        .filter(|arg| !is_type_empty(arg.implicit_id.clone(), types))
        .map(|arg| (&*arg.name, arg.implicit_id.clone()))
}

/// Iterate over the non-empty extension value name/IDs
fn iter_nonempty_extention_values<'exts, 'info, Resolver: TypeResolver>(
    extension_info: &'exts ExtrinsicExtensionInfo<'info, Resolver::TypeId>,
    types: &Resolver,
) -> impl Iterator<Item = (&'exts str, Resolver::TypeId)> {
    extension_info
        .extension_ids
        .iter()
        .filter(|arg| !is_type_empty(arg.id.clone(), types))
        .map(|arg| (&*arg.name, arg.id.clone()))
}

/// Checks to see whether the type being given is empty, ie would require
/// 0 bytes to encode. We use this to skip 0 byte trasnsaction extensions; ones
/// that are mentioned in the metadata but only used in the node side and require
/// no bytes to be given.
fn is_type_empty<Resolver: TypeResolver>(type_id: Resolver::TypeId, types: &Resolver) -> bool {
    struct IsEmptyVisitor<'r, R> {
        types: &'r R,
    }
    impl<'r, R: TypeResolver> scale_type_resolver::ResolvedTypeVisitor<'r> for IsEmptyVisitor<'r, R> {
        type TypeId = R::TypeId;
        type Value = bool;

        // The default ans safe assumption is that a type is _not_ empty.
        fn visit_unhandled(self, _: scale_type_resolver::UnhandledKind) -> Self::Value {
            false
        }
        // Arrays are empty if they are 0 length or the type inside is empty.
        fn visit_array(self, type_id: Self::TypeId, len: usize) -> Self::Value {
            len == 0 || is_type_empty(type_id, self.types)
        }
        // Composites are empty if all of their fields are empty.
        fn visit_composite<Path, Fields>(self, _path: Path, mut fields: Fields) -> Self::Value
        where
            Path: scale_type_resolver::PathIter<'r>,
            Fields: scale_decode::FieldIter<'r, Self::TypeId>,
        {
            fields.all(|f| is_type_empty(f.id, self.types))
        }
        // Tuples are empty if all of their fields are empty.
        fn visit_tuple<TypeIds>(self, mut type_ids: TypeIds) -> Self::Value
        where
            TypeIds: ExactSizeIterator<Item = Self::TypeId>,
        {
            type_ids.all(|id| is_type_empty(id, self.types))
        }
    }

    types
        .resolve_type(type_id, IsEmptyVisitor { types })
        .unwrap_or_default()
}
