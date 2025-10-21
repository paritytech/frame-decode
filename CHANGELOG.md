# Changelog

The format is based on [Keep a Changelog].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/

## 0.11.0 (2025-10-08)

This release adds encode/decode logic for Runtime APIs, Constants and Custom Values, and removes some unused bits and pieces. Additionally we merge the trait functions for getting lists of entries into the main `*TypeInfo` traits, and do a little renaming for consistency. See [#46](https://github.com/paritytech/frame-decode/pull/46) for more information.

## 0.10.0 (2025-08-29)

- Provide information about the default value in `StorageInfo`, if one exists. This may be borrowed, and so adds a lifetime to the `StorageInfo` type (which `.into_owned()` can handle if necessary).

## 0.9.0 (2025-07-24)

- Remove the `_legacy` functions; just use the non suffixed versions which are identical.
- Adds storage key encoding via `frame_decode::storage::encode_storage_key`, `frame_decode::storage::encode_storage_key_to`, and`frame_decode::storage::encode_storage_key_with_info_to`, with supporting traits.
- Rename `prefix` to `encode_prefix` to align with the above.

## 0.8.3 (2025-07-17)

- Make a couple of methods in `crate::extrinsics` return `impl ExactSizeIterator` rather than `impl Iterator`, enabling them to be used with `scale_decode::DecodeAsFields`.

## 0.8.2 (2025-07-16)

- Make the `crate::extrinsics::NamedArg` type public, since it's in the public interface.`

## 0.8.1 (2025-07-15)

- Expose a `crate::legacy_types` module which provides `crate::legacy_types::polkadot::relay_chain()` to access the relay chain types. This is gated behind the "legacy-types" feature which is disabled by default.

## 0.8.0 (2025-05-07)

- Support `frame-metadata` v23. That stabilized V16 metadata, so we implement the relevant traits for that here to support it.

## 0.7.1 (2025-04-23)

- Support `frame-metadata` v20-v21.

## 0.7.0 (2025-03-05)

- Bump `frame-metadata` to latest: 20.0.0

## 0.6.1 (2025-01-30)

- Fix a decoding error where the ranges end at 0 if an extrinsic is 0 bytes in length ([#30](https://github.com/paritytech/frame-decode/pull/30))

## 0.6.0 (2024-11-18)

- Bump frame-metadata to 18.0, scale-decode to 0.16 and scale-value to 0.18 (latest versions at time of release).

## 0.5.0 (2024-10-23)

- Bump scale-decode 0.14, scale-value 0.17 and scale-info v2.11.4 ([#7](https://github.com/paritytech/frame-decode/pull/7))
- Remove unused dependency hex ([#8](https://github.com/paritytech/frame-decode/pull/8))
- Bump frame-metadata from 16.0.0 to 17.0.0 ([#9](https://github.com/paritytech/frame-decode/pull/8))
- Bump scale-info-legacy from 0.2.1 to 0.2.2 ([#10](https://github.com/paritytech/frame-decode/pull/10))

## 0.4.0 (2024-10-21)

- Split `ExtrinsicTypeInfo` trait to get signature and extensions info separately, and support being given an extension version in the latter.
- Remove support for V5 signed extrinsics, which are no longer a thing (see [#3685](https://github.com/paritytech/polkadot-sdk/pull/3685) for context).

## 0.3.0 (2024-09-30)

- Fix `extrinsic.call_range()` and `extensions.range()` functions, and clarify descriptions. 
- Add `extrinsic.call_args_range()` to return the call data arguments.

## 0.2.0 (2024-09-30)

- Consistify helper functions; have `list_storage_entries{_any}` and `type_registry_from_metadata{_any}`, where
  the `any` versions take a `RuntimeMetadata` enum and the others take the specific metadata versions contained within.
- Improve the top level docs.

## 0.1.0 (2024-09-27)

Initial release.