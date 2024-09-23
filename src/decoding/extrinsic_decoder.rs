pub mod v4;

use crate::decoding::extrinsic_type_info::ExtrinsicTypeInfo;
use scale_type_resolver::TypeResolver;
use parity_scale_codec::{Decode, Compact};

// todo add:
// decode_extrinsic_inner => decode_extrinsic.
//
// Then at the top level we can have eg:
// decode_extrinsic_any(bytes, metadata, historic_types, visitor)
// decode_extrinsic_with_v15_metadata
// decode_extrinsic_with_v14_metadata
// decode_extrinsic_with_v13_metadata
//
// These ask for exactly what's needed and no more.
//
// - Add fns on v4 extrinsic to obtain various things.
// - Move v4 module below to separate folder.
// - Add storage decode stuff next.

/// A decoded Extrinsic.
pub enum Extrinsic<'resolver, TypeId> {
    /// A version 4 extrinsic.
    V4(v4::Extrinsic<'resolver, TypeId>)
}

impl <'resolver, TypeId> Extrinsic<'resolver, TypeId> {
    /// Map the extrinsic type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, f: F) -> Extrinsic<'resolver, NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId
    {
        match self {
            Extrinsic::V4(e) => Extrinsic::V4(e.map_type_id(f))
        }
    }
}

/// An error returned trying to decode extrinsic bytes.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum ExtrinsicDecodeError {
    CannotDecodeLength,
    WrongLength { expected_len: usize, actual_len: usize 
    },
    NotEnoughBytes,
    VersionNotSupported { extrinsic_version: u8 },
    V4(v4::ExtrinsicDecodeError)
}

impl core::error::Error for ExtrinsicDecodeError {}

impl core::fmt::Display for ExtrinsicDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExtrinsicDecodeError::CannotDecodeLength => {
                write!(f, "Cannot decode the compact-encoded extrinsic length.")
            },
            ExtrinsicDecodeError::WrongLength { expected_len, actual_len } => {
                write!(f, "The actual number of bytes does not match the compact-encoded extrinsic length; expected {expected_len} bytes but got {actual_len} bytes.")
            },
            ExtrinsicDecodeError::NotEnoughBytes => {
                write!(f, "Not enough bytes to decode a valid extrinsic")
            },
            ExtrinsicDecodeError::VersionNotSupported { extrinsic_version } => {
                write!(f, "This extrinsic version ({extrinsic_version}) is not supported")
            },
            ExtrinsicDecodeError::V4(extrinsic_decode_error) => {
                write!(f, "Cannotdecode version 4 extrinsic:\n\n{extrinsic_decode_error}")
            },
        }
    }
}

/// Given the bytes representing an extrinsic (including the prefixed compact-encoded
/// length), some means to get the extrinsic information, some means to resolve types,
/// and a visitor defining what to decode the bytes into, this returns a decoded [`Extrinsic`]
/// or an [`ExtrinsicDecodeError`] if something went wrong.
/// 
/// The [`Extrinsic`] type is then shows you where different types are in the original bytes,
/// allowing them to be decoded easily using whatever means you prefer.
pub fn decode_extrinsic<'scale, 'info, 'resolver, Info, Resolver>(
    cursor: &mut &'scale [u8], 
    info: &'info Info, 
    type_resolver: &'resolver Resolver,
) -> Result<Extrinsic<'info, Info::TypeId>, ExtrinsicDecodeError> 
where
    Info: ExtrinsicTypeInfo,
    Info::TypeId: core::fmt::Debug + Clone,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    let original_len = cursor.len();
    let ext_len = Compact::<u64>::decode(cursor)
        .map_err(|_| ExtrinsicDecodeError::CannotDecodeLength)?.0 as usize;

    // How many bytes in are we. All ranges calculated need to take this into account.
    let offset = original_len - cursor.len();

    if cursor.len() != ext_len {
        return Err(ExtrinsicDecodeError::WrongLength { expected_len: ext_len, actual_len: cursor.len() })
    }

    if cursor.len() < 1 {
        return Err(ExtrinsicDecodeError::NotEnoughBytes)
    }

    // Decide how to decode the extrinsic based on the version.
    // As of https://github.com/paritytech/polkadot-sdk/pull/3685,
    // only 6 bits used for version. Shouldn't break old impls.
    let version = cursor[0] & 0b0011_1111;

    match version {
        4 => v4::decode_extrinsic(offset, cursor, info, type_resolver)
            .map(Extrinsic::V4)
            .map_err(ExtrinsicDecodeError::V4),
        v => Err(ExtrinsicDecodeError::VersionNotSupported { extrinsic_version: v })
    }
}
