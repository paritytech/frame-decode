# Changelog

The format is based on [Keep a Changelog].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/

## 0.5.0 (2024-10-23)

- Bump scale-decode to 0.14 and scale-value to 0.16 ([#7](https://github.com/paritytech/frame-decode/pull/7))
- Remove unused dependency hex ([#8](https://github.com/paritytech/frame-decode/pull/8))
- Bump frame-metadata from 16.0.0 to 17.0.0 ([#9](https://github.com/paritytech/frame-decode/pull/8))

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