# Changelog

## 0.5.1 (2024-08-28) - `miden-objects` crate only

- Implemented `PrettyPrint` and `Display` for `NoteScript`.

## 0.5.0 (2024-08-27)

### Features

- [BREAKING] Increase of nonce does not require changes in account state any more (#796).
- Changed `AccountCode` procedures from merkle tree to sequential hash + added storage_offset support (#763).
- Implemented merging of account deltas (#797).
- Implemented `create_note` and `move_asset_into_note` basic wallet procedures (#808).
- Made `miden_lib::notes::build_swap_tag()` function public (#817).
- [BREAKING] Changed the `NoteFile::NoteDetails` type to struct and added a `after_block_num` field (#823).

### Changes

- Renamed "consumed" and "created" notes into "input" and "output" respectively (#791).
- [BREAKING] Renamed `NoteType::OffChain` into `NoteType::Private`.
- [BREAKING] Renamed public accessors of the `Block` struct to match the updated fields (#791).
- [BREAKING] Changed the `TransactionArgs` to use `AdviceInputs` (#793).
- Setters in `memory` module don't drop the setting `Word` anymore (#795).
- Added `CHANGELOG.md` warning message on CI (#799).
- Added high-level methods for `MockChain` and related structures (#807).
- [BREAKING] Renamed `NoteExecutionHint` to `NoteExecutionMode` and added new `NoteExecutionHint` to `NoteMetadata` (#812, #816).
- [BREAKING] Changed the interface of the `miden::tx::add_asset_to_note` (#808).
- [BREAKING] Refactored and simplified `NoteOrigin` and `NoteInclusionProof` structs (#810, #814).
- [BREAKING] Refactored account storage and vault deltas (#822).
- Added serialization and equality comparison for `TransactionScript` (#824).
- [BREAKING] Migrated to Miden VM v0.10 (#826).
- Added conversions for `NoteExecutionHint` (#827).

## 0.4.0 (2024-07-03)

### Features

- [BREAKING] Introduce `OutputNote::Partial` variant (#698).
- [BREAKING] Added support for input notes with delayed verification of inclusion proofs (#724, #732, #759, #770, #772).
- Added new `NoteFile` object to represent serialized notes (#721).
- Added transaction IDs to the `Block` struct (#734).
- Added ability for users to set the aux field when creating a note (#752).

### Enhancements

- Replaced `cargo-make` with just `make` for running tasks (#696).
- [BREAKING] Split `Account` struct constructor into `new()` and `from_parts()` (#699).
- Generalized `build_recipient_hash` procedure to build recipient hash for custom notes (#706).
- [BREAKING] Changed the encoding of inputs notes in the advice map for consumed notes (#707).
- Created additional `emit` events for kernel related `.masm` procedures (#708).
- Implemented `build_recipient_hash` procedure to build recipient hash for custom notes (#710).
- Removed the `mock` crate in favor of having mock code behind the `testing` flag in remaining crates (#711).
- [BREAKING] Created `auth` module for `TransactionAuthenticator` and other related objects (#714).
- Added validation for the output stack to make sure it was properly cleaned (#717).
- Made `DataStore` conditionally async using `winter-maybe-async` (#725).
- Changed note pointer from Memory `note_ptr` to `note_index` (#728).
- [BREAKING] Changed rng to mutable reference in note creation functions (#733).
- [BREAKING] Replaced `ToNullifier` trait with `ToInputNoteCommitments`, which includes the `note_id` for delayed note authentication (#732).
- Added `Option<NoteTag>`to `NoteFile` (#741).
- Fixed documentation and added `make doc` CI job (#746).
- Updated and improved [.pre-commit-config.yaml](.pre-commit-config.yaml) file (#748).
- Created `get_serial_number` procedure to get the serial num of the currently processed note (#760).
- [BREAKING] Added support for conversion from `Nullifier` to `InputNoteCommitment`, commitment header return reference (#774).
- Added `compute_inputs_hash` procedure for hash computation of the arbitrary number of note inputs (#750).

## 0.3.1 (2024-06-12)

- Replaced `cargo-make` with just `make` for running tasks (#696).
- Made `DataStore` conditionally async using `winter-maybe-async` (#725)
- Fixed `StorageMap`s implementation and included into apply_delta (#745)

## 0.3.0 (2024-05-14)

- Introduce the `miden-bench-tx` crate used for transactions benchmarking (#577).
- [BREAKING] Removed the transaction script root output from the transaction kernel (#608).
- [BREAKING] Refactored account update details, moved `Block` to `miden-objects` (#618, #621).
- [BREAKING] Made `TransactionExecutor` generic over `TransactionAuthenticator` (#628).
- [BREAKING] Changed type of `version` and `timestamp` fields to `u32`, moved `version` to the beginning of block header (#639).
- [BREAKING] Renamed `NoteEnvelope` into `NoteHeader` and introduced `NoteDetails` (#664).
- [BREAKING] Updated `create_swap_note()` procedure to return `NoteDetails` and defined SWAP note tag format (#665).
- Implemented `OutputNoteBuilder` (#669).
- [BREAKING] Added support for full details of private notes, renamed `OutputNote` variants and changed their meaning (#673).
- [BREAKING] Added `add_asset_to_note` procedure to the transaction kernel (#674).
- Made `TransactionArgs::add_expected_output_note()` more flexible (#681).
- [BREAKING] Enabled support for notes without assets and refactored `create_note` procedure in the transaction kernel (#686).

## 0.2.3 (2024-04-26) - `miden-tx` crate only

- Fixed handling of debug mode in `TransactionExecutor` (#627)

## 0.2.2 (2024-04-23) - `miden-tx` crate only

- Added `with_debug_mode()` methods to `TransactionCompiler` and `TransactionExecutor` (#562).

## 0.2.1 (2024-04-12)

- [BREAKING] Return a reference to `NoteMetadata` from output notes (#593).
- Add more type conversions for `NoteType` (#597).
- Fix note input padding for expected output notes (#598).

## 0.2.0 (2024-04-11)

- [BREAKING] Implement support for public accounts (#481, #485, #538).
- [BREAKING] Implement support for public notes (#515, #540, #572).
- Improved `ProvenTransaction` validation (#532).
- [BREAKING] Updated `no-std` setup (#533).
- Improved `ProvenTransaction` serialization (#543).
- Implemented note tree wrapper structs (#560).
- [BREAKING] Migrated to v0.9 version of Miden VM (#567).
- [BREAKING] Added account storage type parameter to `create_basic_wallet` and `create_basic_fungible_faucet` (miden-lib
  crate only) (#587).
- Removed serialization of source locations from account code (#590).

## 0.1.1 (2024-03-07) - `miden-objects` crate only

- Added `BlockHeader::mock()` method (#511)

## 0.1.0 (2024-03-05)

- Initial release.
