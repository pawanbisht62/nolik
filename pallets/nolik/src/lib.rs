#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, sp_io::offchain_index};
	use frame_system::pallet_prelude::*;
	use nolik_metadata::{Channel, MessageMetadata};
	use scale_info::prelude::vec::Vec;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Too many messages sent
		MessageCounterOverflow,
		/// Message has a bad format
		MessageMalformed,
		/// Message metadata has a bad format
		MetadataMalformed,
	}

	// Events.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new message was sent
		MessageSent { key: Vec<u8>, metadata: MessageMetadata },
	}

	/// Keeps track of a total number of sent messages by all users
	#[pallet::storage]
	#[pallet::getter(fn message_counter)]
	pub(super) type MessageCounter<T> = StorageValue<_, u128, ValueQuery>;

	/// The encoded key is used to store a message in off-chain storage
	#[derive(Debug, Encode, Decode)]
	pub struct MessageKey<'a, T: Config> {
		account: &'a T::AccountId,
		/// Message sequence number
		counter: u128,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send the `message`.
		///
		/// Puts the message to off-chain storage with a unique key and emits an event with the
		/// key and `metadata`.
		///
		/// # Arguments
		///
		/// * `metadata` - Metadata to describe the message and to decrypt it
		/// * `message` - Encrypted message data, possibly having a big size. We pass message as raw
		///   bytes so no encoding and no heap allocation is needed prior to putting the message to
		///   off-chain storage.
		#[pallet::call_index(0)]
		// SBP-M1 review: Implement benchmarking and use benchmarked weight
		#[pallet::weight(10_000)]
		pub fn send_message(
			origin: OriginFor<T>,
			metadata: MessageMetadata,
			// SBP-M1 review: BoundedVec should be used to improve security
			message: Vec<u8>,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			Self::check_message(&message, &metadata)?;

			let counter = MessageCounter::<T>::get();

			// SBP-M1: Can be simplified like this `counter.checked_add(1).ok_or(<Error<T>>::MessageCounterOverflow)?`
			let (counter, overflowed) = counter.overflowing_add(1);
			// u128 should not overflow, practically impossible
			if overflowed {
				Err(<Error<T>>::MessageCounterOverflow)?;
			}

			let key = Self::derived_key(&account, counter - 1);
			// SBP-M1 review: please remove commented code
			// frame_support::log::info!("The offchain key !!! {:02x?}", key);

			// save message to offchain storage
			offchain_index::set(&key, &message);
			// update the message counter
			MessageCounter::<T>::put(counter);
			// emit an event
			Self::deposit_event(Event::MessageSent { key, metadata });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Combines a user account with a message counter to make it unique
		pub fn derived_key(account: &T::AccountId, counter: u128) -> Vec<u8> {
			// e.g. "my_account_id/623451"
			MessageKey::<T> { account, counter }.encode()
		}

		/// Check message format is valid
		pub fn check_message(message: &[u8], metadata: &MessageMetadata) -> DispatchResult {
			if message.is_empty() {
				Err(<Error<T>>::MessageMalformed)?;
			}

			if metadata.channels.is_empty() {
				Err(<Error<T>>::MetadataMalformed)?;
			}

			for Channel { nonce, parties } in &metadata.channels {
				if nonce.is_empty() ||
					parties.is_empty() || parties.len() != metadata.channels.len()
				{
					Err(<Error<T>>::MetadataMalformed)?;
				}

				for part in parties {
					if part.is_empty() {
						Err(<Error<T>>::MetadataMalformed)?;
					}
				}
			}
			Ok(())
		}
	}
}
