use alloc::{collections::BTreeMap, rc::Rc, string::String, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
        AccountCode,
    },
    assets::{Asset, FungibleAsset},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteHeader, NoteId, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    testing::{
        account_code::{
            ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT, ACCOUNT_INCR_NONCE_MAST_ROOT,
            ACCOUNT_REMOVE_ASSET_MAST_ROOT, ACCOUNT_SET_CODE_MAST_ROOT, ACCOUNT_SET_ITEM_MAST_ROOT,
            ACCOUNT_SET_MAP_ITEM_MAST_ROOT,
        },
        constants::{FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
        notes::DEFAULT_NOTE_CODE,
        prepare_word,
        storage::{STORAGE_INDEX_0, STORAGE_INDEX_2},
    },
    transaction::{ProvenTransaction, TransactionArgs, TransactionScript},
    Felt, Word, MIN_PROOF_SECURITY_LEVEL,
};
use miden_prover::ProvingOptions;
use vm_processor::{
    utils::{Deserializable, Serializable},
    Digest, MemAdviceProvider, ONE,
};

use super::{TransactionExecutor, TransactionHost, TransactionProver, TransactionVerifier};
use crate::{testing::TransactionContextBuilder, TransactionMastStore};

mod kernel_tests;

// TESTS
// ================================================================================================

#[test]
fn transaction_executor_witness() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None);

    let account_id = tx_context.account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, tx_context.tx_args().clone())
        .unwrap();

    let tx_inputs = executed_transaction.tx_inputs();
    let tx_args = executed_transaction.tx_args();

    // use the witness to execute the transaction again
    let (stack_inputs, advice_inputs) = TransactionKernel::prepare_inputs(
        tx_inputs,
        tx_args,
        Some(executed_transaction.advice_witness().clone()),
    );
    let mem_advice_provider: MemAdviceProvider = advice_inputs.into();

    // load account/note/tx_script MAST to the mast_store
    let mast_store = Rc::new(TransactionMastStore::new());
    mast_store.load_transaction_code(tx_inputs, tx_args);

    let mut host: TransactionHost<MemAdviceProvider, ()> =
        TransactionHost::new(tx_inputs.account().into(), mem_advice_provider, mast_store, None)
            .unwrap();
    let result = vm_processor::execute(
        &TransactionKernel::main(),
        stack_inputs,
        &mut host,
        Default::default(),
    )
    .unwrap();

    let (advice_provider, _, output_notes, _signatures, _tx_progress) = host.into_parts();
    let (_, map, _) = advice_provider.into_parts();
    let tx_outputs = TransactionKernel::from_transaction_parts(
        result.stack_outputs(),
        &map.into(),
        output_notes,
    )
    .unwrap();

    assert_eq!(executed_transaction.final_account().hash(), tx_outputs.account.hash());
    assert_eq!(executed_transaction.output_notes(), &tx_outputs.output_notes);
}

#[test]
fn executed_transaction_account_delta() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None);
    let account_id = tx_context.tx_inputs().account().id();

    let new_acct_code_src = "\
    export.account_proc_1
        push.9.9.9.9
        dropw
    end
    ";
    let new_acct_code =
        AccountCode::compile(new_acct_code_src, TransactionKernel::assembler_testing()).unwrap();

    // updated storage
    let updated_slot_value = [Felt::new(7), Felt::new(9), Felt::new(11), Felt::new(13)];

    // updated storage map
    let updated_map_key = [Felt::new(14), Felt::new(15), Felt::new(16), Felt::new(17)];
    let updated_map_value = [Felt::new(18), Felt::new(19), Felt::new(20), Felt::new(21)];

    // removed assets
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);
    let removed_assets = [removed_asset_1, removed_asset_2, removed_asset_3];

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_local_use_case(0, 0).unwrap();
    let tag3 = NoteTag::for_local_use_case(0, 0).unwrap();

    let aux1 = Felt::new(27);
    let aux2 = Felt::new(28);
    let aux3 = Felt::new(29);

    let note_type1 = NoteType::Private;
    let note_type2 = NoteType::Private;
    let note_type3 = NoteType::Private;

    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    assert_eq!(tag2.validate(note_type2), Ok(tag2));
    assert_eq!(tag3.validate(note_type3), Ok(tag3));

    let tx_script_src = format!(
        "\
        use.miden::account

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        #TODO: Move this into an account library
        proc.set_item
            push.0 movdn.5 push.0 movdn.5 push.0 movdn.5
            # => [index, V', 0, 0, 0]

            call.{ACCOUNT_SET_ITEM_MAST_ROOT}
            # => [R', V]
        end

        proc.set_map_item
            #push.0 movdn.9 push.0 movdn.9 push.0 movdn.9
            # => [index, KEY, VALUE, 0, 0, 0]

            call.{ACCOUNT_SET_MAP_ITEM_MAST_ROOT}
            # => [R', V]
        end

        proc.set_code
            call.{ACCOUNT_SET_CODE_MAST_ROOT}
            # => [0, 0, 0, 0]

            dropw
            # => []
        end

        proc.incr_nonce
            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            # => [0]

            drop
            # => []
        end

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Update account storage item
            ## ------------------------------------------------------------------------------------
            # push a new value for the storage slot onto the stack
            push.{UPDATED_SLOT_VALUE}
            # => [13, 11, 9, 7]

            # get the index of account storage slot
            push.{STORAGE_INDEX_0}
            # => [idx, 13, 11, 9, 7]

            # update the storage value
            exec.set_item dropw dropw
            # => []

            ## Update account storage map
            ## ------------------------------------------------------------------------------------
            # push a new VALUE for the storage map onto the stack
            push.{UPDATED_MAP_VALUE}
            # => [18, 19, 20, 21]

            # push a new KEY for the storage map onto the stack
            push.{UPDATED_MAP_KEY}
            # => [14, 15, 16, 17, 18, 19, 20, 21]

            # get the index of account storage slot
            push.{STORAGE_INDEX_2}
            # => [idx, 14, 15, 16, 17, 18, 19, 20, 21]

            # update the storage value
            exec.set_map_item dropw dropw dropw
            # => []

            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{EXECUTION_HINT_1} # note_execution_hint
            push.{NOTETYPE1}        # note_type
            push.{aux1}             # aux
            push.{tag1}             # tag
            push.{REMOVED_ASSET_1}  # asset
            # => [ASSET, tag, aux, note_type, RECIPIENT]

            call.::miden::contracts::wallets::basic::send_asset dropw dropw dropw dropw
            # => []

            # totally deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{EXECUTION_HINT_2} # note_execution_hint
            push.{NOTETYPE2}        # note_type
            push.{aux2}             # aux
            push.{tag2}             # tag
            push.{REMOVED_ASSET_2}  # asset
            # => [ASSET, tag, aux, note_type, RECIPIENT]

            call.::miden::contracts::wallets::basic::send_asset dropw dropw dropw dropw
            # => []

            # send non-fungible asset
            push.0.1.2.3            # recipient
            push.{EXECUTION_HINT_3} # note_execution_hint
            push.{NOTETYPE3}        # note_type
            push.{aux3}             # aux
            push.{tag3}             # tag
            push.{REMOVED_ASSET_3}  # asset
            # => [ASSET, tag, aux, note_type, RECIPIENT]
            
            call.::miden::contracts::wallets::basic::send_asset dropw dropw dropw dropw
            # => []

            ## Update account code
            ## ------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_COMMITMENT} exec.set_code dropw
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce drop
            # => []
        end
    ",
        NEW_ACCOUNT_COMMITMENT = prepare_word(&new_acct_code.commitment()),
        UPDATED_SLOT_VALUE = prepare_word(&Word::from(updated_slot_value)),
        UPDATED_MAP_VALUE = prepare_word(&Word::from(updated_map_value)),
        UPDATED_MAP_KEY = prepare_word(&Word::from(updated_map_key)),
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
        NOTETYPE3 = note_type3 as u8,
        EXECUTION_HINT_1 = Felt::from(NoteExecutionHint::always()),
        EXECUTION_HINT_2 = Felt::from(NoteExecutionHint::none()),
        EXECUTION_HINT_3 = Felt::from(NoteExecutionHint::on_block_slot(1, 1, 1)),
    );

    let tx_script =
        TransactionScript::compile(tx_script_src, [], TransactionKernel::assembler_testing())
            .unwrap();
    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    // We expect one updated item and one updated map
    assert_eq!(executed_transaction.account_delta().storage().slots().len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().slots().get(&STORAGE_INDEX_0),
        Some(&updated_slot_value)
    );

    assert_eq!(executed_transaction.account_delta().storage().maps().len(), 1);
    assert_eq!(
        executed_transaction
            .account_delta()
            .storage()
            .maps()
            .get(&STORAGE_INDEX_2)
            .unwrap()
            .leaves(),
        &Some((updated_map_key.into(), updated_map_value))
            .into_iter()
            .collect::<BTreeMap<Digest, _>>()
    );

    // vault delta
    // --------------------------------------------------------------------------------------------
    // assert that added assets are tracked
    let added_assets = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .find(|n| n.note().assets().num_assets() == 3)
        .unwrap()
        .note()
        .assets()
        .iter()
        .cloned()
        .collect::<Vec<_>>();

    assert!(executed_transaction
        .account_delta()
        .vault()
        .added_assets()
        .all(|x| added_assets.contains(&x)));
    assert_eq!(
        added_assets.len(),
        executed_transaction.account_delta().vault().added_assets().count()
    );

    // assert that removed assets are tracked
    assert!(executed_transaction
        .account_delta()
        .vault()
        .removed_assets()
        .all(|x| removed_assets.contains(&x)));
    assert_eq!(
        removed_assets.len(),
        executed_transaction.account_delta().vault().removed_assets().count()
    );
}

#[test]
fn test_empty_delta_nonce_update() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None);
    let account_id = tx_context.tx_inputs().account().id();

    let tx_script_src = format!(
        "\
        begin
            push.1

            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            # => [0, 1]

            drop drop
            # => []
        end
    "
    );

    let tx_script =
        TransactionScript::compile(tx_script_src, [], TransactionKernel::assembler_testing())
            .unwrap();
    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    // We expect one updated item and one updated map
    assert_eq!(executed_transaction.account_delta().storage().slots().len(), 0);

    assert_eq!(executed_transaction.account_delta().storage().maps().len(), 0);
}

#[test]
fn test_send_note_proc() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(tx_context.clone(), None).with_debug_mode(true);
    let account_id = tx_context.tx_inputs().account().id();

    // removed assets
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);

    let tag = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let aux = Felt::new(27);
    let note_type = NoteType::Private;

    assert_eq!(tag.validate(note_type), Ok(tag));

    // prepare the asset vector to be removed for each test variant
    let assets_matrix = vec![
        vec![],
        vec![removed_asset_1],
        vec![removed_asset_1, removed_asset_2],
        vec![removed_asset_1, removed_asset_2, removed_asset_3],
    ];

    for removed_assets in assets_matrix {
        // Prepare the string containing the procedures required for adding assets to the note.
        // Depending on the number of the assets to remove, the resulting string will be extended
        // with the corresponding number of procedure "blocks"
        let mut assets_to_remove = String::new();
        for asset in removed_assets.iter() {
            assets_to_remove.push_str(&format!(
                "\n
            # prepare the stack for the next call
            dropw

            # push the asset to be removed
            push.{ASSET}
            # => [ASSET, note_idx, GARBAGE(11)]

            call.wallet::move_asset_to_note
            # => [ASSET, note_idx, GARBAGE(11)]\n",
                ASSET = prepare_word(&asset.into())
            ))
        }

        let tx_script_src = format!(
            "\
            use.miden::account
            use.miden::contracts::wallets::basic->wallet
            use.miden::tx

            ## ACCOUNT PROCEDURE WRAPPERS
            ## ========================================================================================
            proc.incr_nonce
                call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
                # => [0]

                drop
                # => []
            end

            ## TRANSACTION SCRIPT
            ## ========================================================================================
            begin
                # prepare the values for note creation
                push.1.2.3.4      # recipient
                push.1            # note_execution_hint (NoteExecutionHint::Always)
                push.{note_type}  # note_type
                push.{aux}        # aux
                push.{tag}        # tag
                # => [tag, aux, note_type, RECIPIENT, ...]

                # pad the stack with zeros before calling the `create_note`.
                padw padw swapdw
                # => [tag, aux, execution_hint, note_type, RECIPIENT, PAD(8) ...]

                call.wallet::create_note
                # => [note_idx, GARBAGE(15)]

                movdn.4
                # => [GARBAGE(4), note_idx, GARBAGE(11)]

                {assets_to_remove}

                dropw dropw dropw dropw

                ## Update the account nonce
                ## ------------------------------------------------------------------------------------
                push.1 exec.incr_nonce drop
                # => []
            end
        ",
            note_type = note_type as u8,
        );

        let tx_script =
            TransactionScript::compile(tx_script_src, [], TransactionKernel::assembler_testing())
                .unwrap();
        let tx_args = TransactionArgs::new(
            Some(tx_script),
            None,
            tx_context.tx_args().advice_inputs().clone().map,
        );

        let block_ref = tx_context.tx_inputs().block_header().block_num();
        let note_ids = tx_context
            .tx_inputs()
            .input_notes()
            .iter()
            .map(|note| note.id())
            .collect::<Vec<_>>();

        // expected delta
        // --------------------------------------------------------------------------------------------
        // execute the transaction and get the witness
        let executed_transaction =
            executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

        // nonce delta
        // --------------------------------------------------------------------------------------------
        assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

        // vault delta
        // --------------------------------------------------------------------------------------------
        // assert that removed assets are tracked
        assert!(executed_transaction
            .account_delta()
            .vault()
            .removed_assets()
            .all(|x| removed_assets.contains(&x)));
        assert_eq!(
            removed_assets.len(),
            executed_transaction.account_delta().vault().removed_assets().count()
        );
    }
}

#[test]
fn executed_transaction_output_notes() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved_with_account_vault_delta()
        .build();

    let executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(tx_context.clone(), None).with_debug_mode(true);
    let account_id = tx_context.tx_inputs().account().id();

    // removed assets
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let combined_asset = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);
    let removed_asset_4 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionMode::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let tag3 = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let aux1 = Felt::new(27);
    let aux2 = Felt::new(28);
    let aux3 = Felt::new(29);

    let note_type1 = NoteType::Private;
    let note_type2 = NoteType::Public;
    let note_type3 = NoteType::Public;

    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    assert_eq!(tag2.validate(note_type2), Ok(tag2));
    assert_eq!(tag3.validate(note_type3), Ok(tag3));

    // In this test we create 3 notes. Note 1 is private, Note 2 is public and Note 3 is public
    // without assets.

    // Create the expected output note for Note 2 which is public
    let serial_num_2 = Word::from([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]);
    let note_script_2 =
        NoteScript::compile(DEFAULT_NOTE_CODE, TransactionKernel::assembler_testing()).unwrap();
    let inputs_2 = NoteInputs::new(vec![]).unwrap();
    let metadata_2 =
        NoteMetadata::new(account_id, note_type2, tag2, NoteExecutionHint::none(), aux2).unwrap();
    let vault_2 = NoteAssets::new(vec![removed_asset_3, removed_asset_4]).unwrap();
    let recipient_2 = NoteRecipient::new(serial_num_2, note_script_2, inputs_2);
    let expected_output_note_2 = Note::new(vault_2, metadata_2, recipient_2);

    // Create the expected output note for Note 3 which is public
    let serial_num_3 = Word::from([Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)]);
    let note_script_3 =
        NoteScript::compile(DEFAULT_NOTE_CODE, TransactionKernel::assembler_testing()).unwrap();
    let inputs_3 = NoteInputs::new(vec![]).unwrap();
    let metadata_3 = NoteMetadata::new(
        account_id,
        note_type3,
        tag3,
        NoteExecutionHint::on_block_slot(1, 2, 3),
        aux3,
    )
    .unwrap();
    let vault_3 = NoteAssets::new(vec![]).unwrap();
    let recipient_3 = NoteRecipient::new(serial_num_3, note_script_3, inputs_3);
    let expected_output_note_3 = Note::new(vault_3, metadata_3, recipient_3);

    let tx_script_src = format!(
        "\
        use.miden::account
        use.miden::contracts::wallets::basic->wallet

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        proc.create_note
            # pad the stack before the syscall to prevent accidental modification of the deeper stack
            # elements
            padw padw swapdw
            # => [tag, aux, execution_hint, note_type, RECIPIENT, PAD(8)]

            call.wallet::create_note
            # => [note_idx, PAD(15)]

            # remove excess PADs from the stack
            swapdw dropw dropw movdn.7 dropw drop drop drop
            # => [note_idx]
        end

        proc.add_asset_to_note
            # pad the stack before the syscall to prevent accidental modification of the deeper stack
            # elements
            push.0.0.0 padw padw swapdw movup.7 swapw
            # => [ASSET, note_idx, PAD(11)]

            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            # => [ASSET, note_idx, PAD(11)]

            # remove excess PADs from the stack
            swapdw dropw dropw swapw movdn.7 drop drop drop
            # => [ASSET, note_idx]

            dropw
            # => [note_idx]
        end

        proc.remove_asset
            call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
            # => [note_ptr]
        end

        proc.incr_nonce
            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            # => [0]

            drop
            # => []
        end

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3                        # recipient
            push.{EXECUTION_HINT_1}             # note execution hint
            push.{NOTETYPE1}                    # note_type
            push.{aux1}                         # aux
            push.{tag1}                         # tag
            exec.create_note
            # => [note_idx]
            push.{REMOVED_ASSET_1}              # asset
            exec.remove_asset
            # => [ASSET, note_ptr]
            exec.add_asset_to_note
            # => [note_idx]

            push.{REMOVED_ASSET_2}              # asset_2
            exec.remove_asset
            # => [ASSET, note_ptr]
            exec.add_asset_to_note drop
            # => []

            # send non-fungible asset
            push.{RECIPIENT2}                   # recipient
            push.{EXECUTION_HINT_2}             # note execution hint
            push.{NOTETYPE2}                    # note_type
            push.{aux2}                         # aux
            push.{tag2}                         # tag
            exec.create_note
            # => [note_idx]

            push.{REMOVED_ASSET_3}              # asset_3
            exec.remove_asset
            exec.add_asset_to_note
            # => [note_idx]

            push.{REMOVED_ASSET_4}              # asset_4
            exec.remove_asset
            # => [ASSET, note_idx]
            exec.add_asset_to_note drop
            # => []

            # create a public note without assets
            push.{RECIPIENT3}                   # recipient
            push.{EXECUTION_HINT_3}             # note execution hint
            push.{NOTETYPE3}                    # note_type
            push.{aux3}                         # aux
            push.{tag3}                         # tag
            exec.create_note drop
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce
            # => []
        end
    ",
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        REMOVED_ASSET_4 = prepare_word(&Word::from(removed_asset_4)),
        RECIPIENT2 = prepare_word(&Word::from(expected_output_note_2.recipient().digest())),
        RECIPIENT3 = prepare_word(&Word::from(expected_output_note_3.recipient().digest())),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
        NOTETYPE3 = note_type3 as u8,
        EXECUTION_HINT_1 = Felt::from(NoteExecutionHint::always()),
        EXECUTION_HINT_2 = Felt::from(NoteExecutionHint::none()),
        EXECUTION_HINT_3 = Felt::from(NoteExecutionHint::on_block_slot(11, 22, 33)),
    );

    let tx_script =
        TransactionScript::compile(tx_script_src, [], TransactionKernel::assembler_testing())
            .unwrap();
    let mut tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_args.add_expected_output_note(&expected_output_note_2);
    tx_args.add_expected_output_note(&expected_output_note_3);

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness

    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // output notes
    // --------------------------------------------------------------------------------------------
    let output_notes = executed_transaction.output_notes();

    // assert that the expected output note is present
    // NOTE: the mock state already contains 3 output notes
    assert_eq!(output_notes.num_notes(), 6);

    let output_note_id_3 = executed_transaction.output_notes().get_note(3).id();
    let recipient_3 = Digest::from([Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)]);
    let note_assets_3 = NoteAssets::new(vec![combined_asset]).unwrap();
    let expected_note_id_3 = NoteId::new(recipient_3, note_assets_3.commitment());
    assert_eq!(output_note_id_3, expected_note_id_3);

    // assert that the expected output note 2 is present
    let output_note = executed_transaction.output_notes().get_note(4);
    let note_id = expected_output_note_2.id();
    let note_metadata = expected_output_note_2.metadata();
    assert_eq!(NoteHeader::from(output_note), NoteHeader::new(note_id, *note_metadata));

    // assert that the expected output note 3 is present and has no assets
    let output_note_3 = executed_transaction.output_notes().get_note(5);
    assert_eq!(expected_output_note_3.id(), output_note_3.id());
    assert_eq!(expected_output_note_3.assets(), output_note_3.assets().unwrap());
}

#[test]
fn prove_witness_and_verify() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    let account_id = tx_context.tx_inputs().account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None);
    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, tx_context.tx_args().clone())
        .unwrap();
    let executed_transaction_id = executed_transaction.id();

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    let serialized_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialized_transaction).unwrap();
    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);
    assert!(verifier.verify(proven_transaction).is_ok());
}

// TEST TRANSACTION SCRIPT
// ================================================================================================

#[test]
fn test_tx_script() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();
    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None);

    let account_id = tx_context.tx_inputs().account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_input_key = [Felt::new(9999), Felt::new(8888), Felt::new(9999), Felt::new(8888)];
    let tx_script_input_value = [Felt::new(9), Felt::new(8), Felt::new(7), Felt::new(6)];
    let tx_script_src = format!(
        "
    begin
        # push the tx script input key onto the stack
        push.{key}

        # load the tx script input value from the map and read it onto the stack
        adv.push_mapval push.16073 drop         # FIX: wrap the decorator to ensure MAST uniqueness
        adv_loadw

        # assert that the value is correct
        push.{value} assert_eqw
    end
",
        key = prepare_word(&tx_script_input_key),
        value = prepare_word(&tx_script_input_value)
    );

    let tx_script = TransactionScript::compile(
        tx_script_src,
        [(tx_script_input_key, tx_script_input_value.into())],
        TransactionKernel::assembler_testing(),
    )
    .unwrap();
    let tx_args = TransactionArgs::new(
        Some(tx_script),
        None,
        tx_context.tx_args().advice_inputs().clone().map,
    );

    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args);

    assert!(
        executed_transaction.is_ok(),
        "Transaction execution failed {:?}",
        executed_transaction,
    );
}
