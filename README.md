# frame-decode

Decode extrinsics, storage keys and storage values from modern or historic Substrate runtimes.

See https://docs.rs/frame-decode/latest/frame_decode/ for more documentation and examples.

# Examples

## Decoding historic extrinsics

```rust
use frame_decode::extrinsics::decode_extrinsic_legacy;
use frame_metadata::RuntimeMetadata;
use parity_scale_codec::Decode;
use scale_info_legacy::ChainTypeRegistry;

let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };

let extrinsics_bytes = std::fs::read("artifacts/exts_5000000_30.json").unwrap();
let extrinsics_hex: Vec<String> = serde_json::from_slice(&extrinsics_bytes).unwrap();

// For historic types, we also need to provide type definitions, since they aren't in the
// metadata. We use scale-info-legacy to do this, and have already defined types for the
// Polkadot relay chain, so let's load those in:
let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();

// We configure the loaded types for the spec version of the extrinsics we want to decode,
// because types can vary between different spec versions.
let mut historic_types_for_spec = historic_types.for_spec_version(30);

// We also want to embelish these types with information from the metadata itself. This avoids
// needing to hardcode a load of type definitions that we can already construct from the metadata.
let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
historic_types_for_spec.prepend(types_from_metadata);

for ext_hex in extrinsics_hex {
    let ext_bytes = hex::decode(ext_hex.trim_start_matches("0x")).unwrap();

    // Decode the extrinsic, returning information about it:
    let ext_info = decode_extrinsic_legacy(&mut &*ext_bytes, &metadata, &historic_types_for_spec).unwrap();

    // Decode the signature details to scale_value::Values.
    if let Some(sig) = ext_info.signature_payload() {
        let address_bytes =  &ext_bytes[sig.address_range()];
        let address_value = decode_with_visitor(
            &mut &*address_bytes,
            *sig.address_type(),
            &metadata.types,
            ValueVisitor::new()
        ).unwrap();

        let signature_bytes = &ext_bytes[sig.signature_range()];
        let signature_value = decode_with_visitor(
            &mut &*signature_bytes,
            *sig.signature_type(),
            &metadata.types,
            ValueVisitor::new()
        ).unwrap();
    }

    // Decode the transaction extensions to scale_value::Values.
    if let Some(exts) = ext_info.transaction_extension_payload() {
        for ext in exts.iter() {
            let ext_name = ext.name();
            let ext_bytes = &ext_bytes[ext.range()];
            let ext_value = decode_with_visitor(
                &mut &*ext_bytes,
                *ext.ty(),
                &metadata.types,
                ValueVisitor::new()
            ).unwrap();
        }
    }

    // Decode the call data args to scale_value::Values.
    for arg in ext_info.call_data() {
        let arg_name = arg.name();
        let arg_bytes = &ext_bytes[arg.range()];
        let arg_value = decode_with_visitor(
            &mut &*arg_bytes,
            *arg.ty(),
            &metadata.types,
            ValueVisitor::new()
        ).unwrap();
    }
}
```

## Decoding historic storage keys

```rust
use frame_decode::storage::decode_storage_key_legacy;
use frame_metadata::RuntimeMetadata;
use parity_scale_codec::Decode;
use scale_info_legacy::ChainTypeRegistry;

let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
 
let storage_keyval_bytes = std::fs::read("artifacts/storage_5000000_30_staking_validators.json").unwrap();
let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();

// For historic types, we also need to provide type definitions, since they aren't in the
// metadata. We use scale-info-legacy to do this, and have already defined types for the
// Polkadot relay chain, so let's load those in:
let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();

// We configure the loaded types for the spec version of the extrinsics we want to decode,
// because types can vary between different spec versions.
let mut historic_types_for_spec = historic_types.for_spec_version(30);

// We also want to embelish these types with information from the metadata itself. This avoids
// needing to hardcode a load of type definitions that we can already construct from the metadata.
let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
historic_types_for_spec.prepend(types_from_metadata);

for (key, _val) in storage_keyval_hex {
    let key_bytes = hex::decode(key.trim_start_matches("0x")).unwrap();

    // Decode the storage key, returning information about it:
    let storage_info = decode_storage_key_legacy(
        "Staking",
        "Validators",
        &mut &*key_bytes,
        &metadata,
        &historic_types_for_spec
    ).unwrap();

   for part in storage_info.parts() {
       // Access information about the hasher for this part of the key:
       let hash_bytes = &key_bytes[part.hash_range()];
       let hasher = part.hasher();

       // If the value is encoded as part of the hasher, we can find and
       // decode the value too:
       if let Some(value_info) = part.value() {
           let value_bytes = &key_bytes[value_info.range()];
           let value = decode_with_visitor(
               &mut &*value_bytes,
               *value_info.ty(),
               &metadata.types,
               ValueVisitor::new()
           ).unwrap();
       }
   }
}
```

# Decoding historic storage values

```rust
use frame_decode::storage::decode_storage_value_legacy;
use frame_metadata::RuntimeMetadata;
use parity_scale_codec::Decode;
use scale_info_legacy::ChainTypeRegistry;
use scale_value::scale::ValueVisitor;

let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
 
let storage_keyval_bytes = std::fs::read("artifacts/storage_5000000_30_staking_validators.json").unwrap();
let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();

// For historic types, we also need to provide type definitions, since they aren't in the
// metadata. We use scale-info-legacy to do this, and have already defined types for the
// Polkadot relay chain, so let's load those in:
let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();

// We configure the loaded types for the spec version of the extrinsics we want to decode,
// because types can vary between different spec versions.
let mut historic_types_for_spec = historic_types.for_spec_version(30);

// We also want to embelish these types with information from the metadata itself. This avoids
// needing to hardcode a load of type definitions that we can already construct from the metadata.
let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
historic_types_for_spec.prepend(types_from_metadata);

for (_key, val) in storage_keyval_hex {
    let value_bytes = hex::decode(val.trim_start_matches("0x")).unwrap();

    // Decode the storage value, here into a scale_value::Value:
    let account_value = decode_storage_value_legacy(
        "Staking",
        "Validators",
        &mut &*value_bytes,
        &metadata,
        &historic_types_for_spec,
        ValueVisitor::new()
    ).unwrap();
}
```