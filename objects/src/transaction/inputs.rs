use core::cell::OnceCell;

use super::{BlockHeader, ChainMmr, Digest, Felt, Hasher, Word, MAX_INPUT_NOTES_PER_TRANSACTION};
use crate::{
    accounts::{validate_account_seed, Account},
    notes::{Note, NoteInclusionProof, NoteOrigin, Nullifier},
    utils::{
        collections::{self, BTreeSet, Vec},
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
        string::ToString,
    },
    TransactionError,
};

// TRANSACTION INPUTS
// ================================================================================================

/// Contains the data required to execute a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionInputs {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub input_notes: InputNotes,
}

impl TransactionInputs {
    /// Validates that a valid account seed has been provided for new accounts.
    ///
    /// # Errors
    /// Returns an error if:
    /// - For a new account, account seed is not provided or the provided seed is invalid.
    /// - For an existing account, account seed was provided.
    pub fn validate_new_account_seed(&self) -> Result<(), TransactionError> {
        match (self.account.is_new(), self.account_seed) {
            (true, Some(seed)) => validate_account_seed(&self.account, seed)
                .map_err(TransactionError::InvalidAccountSeed),
            (true, None) => Err(TransactionError::AccountSeedNoteProvidedForNewAccount),
            (false, Some(_)) => Err(TransactionError::AccountSeedProvidedForExistingAccount),
            (false, None) => Ok(()),
        }
    }
}

// INPUT NOTES
// ================================================================================================

/// Contains a list of input notes for a transaction.
///
/// The list can be empty if the transaction does not consume any notes.
#[derive(Debug, Clone)]
pub struct InputNotes {
    notes: Vec<InputNote>,
    commitment: OnceCell<Digest>,
}

impl InputNotes {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [InputNotes] instantiated from the provided vector of notes.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The total number of notes is greater than 1024.
    /// - The vector of notes contains duplicates.
    pub fn new(notes: Vec<InputNote>) -> Result<Self, TransactionError> {
        if notes.len() > MAX_INPUT_NOTES_PER_TRANSACTION {
            return Err(TransactionError::TooManyInputNotes {
                max: MAX_INPUT_NOTES_PER_TRANSACTION,
                actual: notes.len(),
            });
        }

        let mut seen_notes = BTreeSet::new();
        for note in notes.iter() {
            if !seen_notes.insert(note.note().hash()) {
                return Err(TransactionError::DuplicateInputNote(note.note().hash()));
            }
        }

        Ok(Self { notes, commitment: OnceCell::new() })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to these input notes.
    pub fn commitment(&self) -> Digest {
        *self.commitment.get_or_init(|| build_input_notes_commitment(self.nullifiers()))
    }

    /// Returns total number of input notes.
    pub fn num_notes(&self) -> usize {
        self.notes.len()
    }

    /// Returns true if this [InputNotes] does not contain any notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Returns a reference to the [InputNote] located at the specified index.
    pub fn get_note(&self, idx: usize) -> &InputNote {
        &self.notes[idx]
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over notes in this [InputNotes].
    pub fn iter(&self) -> impl Iterator<Item = &InputNote> {
        self.notes.iter()
    }

    /// Returns an iterator over nullifiers of all notes in this [InputNotes].
    pub fn nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.notes.iter().map(|note| note.note().nullifier())
    }
}

impl IntoIterator for InputNotes {
    type Item = InputNote;
    type IntoIter = collections::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.notes.into_iter()
    }
}

impl PartialEq for InputNotes {
    fn eq(&self, other: &Self) -> bool {
        self.notes == other.notes
    }
}

impl Eq for InputNotes {}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNotes {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // assert is OK here because we enforce max number of notes in the constructor
        assert!(self.notes.len() <= u16::MAX.into());
        target.write_u16(self.notes.len() as u16);
        self.notes.write_into(target);
    }
}

impl Deserializable for InputNotes {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_notes = source.read_u16()?;
        let notes = InputNote::read_batch_from(source, num_notes.into())?;
        Self::new(notes).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Returns the commitment to the input notes represented by the specified nullifiers.
///
/// This is a sequential hash of all (nullifier, ZERO) pairs for the notes consumed in the
/// transaction.
pub fn build_input_notes_commitment<I: Iterator<Item = Nullifier>>(nullifiers: I) -> Digest {
    let mut elements: Vec<Felt> = Vec::new();
    for nullifier in nullifiers {
        elements.extend_from_slice(nullifier.as_elements());
        elements.extend_from_slice(&Word::default());
    }
    Hasher::hash_elements(&elements)
}

// INPUT NOTE
// ================================================================================================

/// An input note for a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct InputNote {
    note: Note,
    proof: NoteInclusionProof,
}

impl InputNote {
    /// Returns a new instance of an [InputNote] with the specified note and proof.
    pub fn new(note: Note, proof: NoteInclusionProof) -> Self {
        Self { note, proof }
    }

    /// Returns a reference to the underlying note.
    pub fn note(&self) -> &Note {
        &self.note
    }

    /// Returns a reference to the inclusion proof of the note.
    pub fn proof(&self) -> &NoteInclusionProof {
        &self.proof
    }

    /// Returns a reference to the origin of the note.
    pub fn origin(&self) -> &NoteOrigin {
        self.proof.origin()
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNote {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.note.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for InputNote {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let note = Note::read_from(source)?;
        let proof = NoteInclusionProof::read_from(source)?;

        Ok(Self { note, proof })
    }
}
