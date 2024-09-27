mod decode_with_error_tracing;
mod list_storage_entries;
mod type_registry_from_metadata;

pub use decode_with_error_tracing::{decode_with_error_tracing, DecodeErrorTrace};
pub use list_storage_entries::{list_storage_entries, StorageEntry};
#[cfg(feature = "legacy")]
pub use type_registry_from_metadata::type_registry_from_metadata;

/// A utility function to unwrap the [`DecodeDifferent`] enum found in earlier metadata versions.
#[cfg(feature = "legacy")]
pub fn as_decoded<A, B>(item: &frame_metadata::decode_different::DecodeDifferent<A, B>) -> &B {
    match item {
        frame_metadata::decode_different::DecodeDifferent::Encode(_a) => {
            panic!("Expecting decoded data")
        }
        frame_metadata::decode_different::DecodeDifferent::Decoded(b) => b,
    }
}

pub trait InfoAndResolver {
    type Info;
    type Resolver;

    fn info(&self) -> &Self::Info;
    fn resolver(&self) -> &Self::Resolver;
}

impl InfoAndResolver for frame_metadata::v14::RuntimeMetadataV14 {
    type Info = frame_metadata::v14::RuntimeMetadataV14;
    type Resolver = scale_info::PortableRegistry;

    fn info(&self) -> &Self::Info {
        self
    }
    fn resolver(&self) -> &Self::Resolver {
        &self.types
    }
}

impl InfoAndResolver for frame_metadata::v15::RuntimeMetadataV15 {
    type Info = frame_metadata::v15::RuntimeMetadataV15;
    type Resolver = scale_info::PortableRegistry;

    fn info(&self) -> &Self::Info {
        self
    }
    fn resolver(&self) -> &Self::Resolver {
        &self.types
    }
}
