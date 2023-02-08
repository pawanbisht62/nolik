use crate::messages::{Message, MessageEntry};
use thiserror::Error;

use sodiumoxide::crypto::{
	box_,
	box_::{Nonce, PublicKey, SecretKey},
};

#[derive(Error, Debug)]
pub enum CypherError {
	#[error("Could not decrypt data for {0:?}")]
	DecryptionFailed(PublicKey),
	#[error("Could not parse nonce {0:?}")]
	InvalidNonce(Vec<u8>),
	#[error("Could not parse pubkey {0:?}")]
	InvalidPubkey(Vec<u8>),
}

pub trait Cypher
where
	Self: Sized,
{
	fn encrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Self;
	fn decrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Result<Self, CypherError>;
}

impl Cypher for Message {
	fn encrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Self {
		Message { entries: self.entries.iter().map(|x| x.encrypt(nonce, pk, sk)).collect() }
	}

	fn decrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Result<Self, CypherError> {
		Ok(Message {
			entries: self
				.entries
				.iter()
				.map(|x| x.decrypt(nonce, pk, sk))
				.into_iter()
				.collect::<Result<_, _>>()?,
		})
	}
}

impl Cypher for MessageEntry {
	fn encrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Self {
		MessageEntry {
			key: self.key.encrypt(nonce, pk, sk),
			value: self.value.encrypt(nonce, pk, sk),
			kind: self.kind.clone(),
		}
	}

	fn decrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Result<Self, CypherError> {
		Ok(MessageEntry {
			key: self.key.decrypt(nonce, pk, sk)?,
			value: self.value.decrypt(nonce, pk, sk)?,
			kind: self.kind.clone(),
		})
	}
}

pub trait BytesCypher {
	fn encrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Vec<u8>;

	fn decrypt(
		&self,
		nonce: &Nonce,
		pk: &PublicKey,
		sk: &SecretKey,
	) -> Result<Vec<u8>, CypherError>;
}

impl BytesCypher for [u8] {
	fn encrypt(&self, nonce: &Nonce, pk: &PublicKey, sk: &SecretKey) -> Vec<u8> {
		box_::seal(self, nonce, pk, sk)
	}

	fn decrypt(
		&self,
		nonce: &Nonce,
		pk: &PublicKey,
		sk: &SecretKey,
	) -> Result<Vec<u8>, CypherError> {
		box_::open(self, nonce, pk, sk).map_err(|_| CypherError::DecryptionFailed(*pk))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::messages::MessageType;
	use sodiumoxide::crypto::box_;

	#[test]
	fn encrypt_decrypt_message() {
		// Encryption and decryption using a Diffie-Hellman algorithm
		let (sender_pk, sender_sk) = box_::gen_keypair();
		let (receiver_pk, receiver_sk) = box_::gen_keypair();

		let nonce = box_::gen_nonce();

		let message = Message {
			entries: vec![MessageEntry {
				key: "key".into(),
				value: "value".into(),
				kind: MessageType::default(),
			}],
		};

		let encrypted_message = message.encrypt(&nonce, &receiver_pk, &sender_sk);
		let decrypted_message = encrypted_message
			.decrypt(&nonce, &sender_pk, &receiver_sk)
			.expect("could not decrypt a test message");

		assert_eq!(message, decrypted_message);
	}
}
