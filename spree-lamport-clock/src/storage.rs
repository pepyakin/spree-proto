//! Utilities for dealing with storage of this SPREE module.
//!
//! There are only two fields exist at the moment:
//! - `timestamp: Timestamp`
//! - `message_queue: Vec<TargetedMsg>`

pub use message_queue::{enqueue_msg, take_queue};
pub use timestamp::next_timestamp;

mod timestamp {
	use crate::ext;
	use codec::{Decode, Encode};
	use primitives::Timestamp;
	const KEY_CURRENT_TIMESTAMP: &[u8] = b":current_timestamp";

	pub fn current_timestamp() -> Timestamp {
		ext::storage_read(KEY_CURRENT_TIMESTAMP)
			.and_then(|raw_timestamp| Timestamp::decode(&mut &raw_timestamp[..]).ok())
			.unwrap_or(0)
	}

	pub fn set_current_timestamp(timestamp: Timestamp) {
		timestamp.using_encoded(|raw_timestamp| {
			ext::storage_write(KEY_CURRENT_TIMESTAMP, raw_timestamp);
		});
	}

	pub fn next_timestamp() -> Timestamp {
		let next = current_timestamp() + 1;
		set_current_timestamp(next);
		next
	}
}

mod message_queue {
	// Gotcha, it is actually a stack and a terribly inefficient implementation.
	use crate::ext;
	use codec::{Decode, Encode};
	use primitives::TargetedMsg;
	const KEY_QUEUE: &[u8] = b":stack";

	fn read_queue() -> Vec<TargetedMsg> {
		ext::storage_read(KEY_QUEUE)
			.and_then(|raw_queue| <Vec<TargetedMsg>>::decode(&mut &raw_queue[..]).ok())
			.unwrap_or_else(Vec::new)
	}

	fn write_queue(queue: Vec<TargetedMsg>) {
		queue.using_encoded(|raw_queue| {
			ext::storage_write(KEY_QUEUE, raw_queue);
		});
	}

	/// Enqueue a given message into the queue.
	pub fn enqueue_msg(msg: TargetedMsg) {
		let mut msgs = read_queue();
		msgs.push(msg);
		write_queue(msgs);
	}

	/// Empty the queue returning its contents.
	///
	/// Returns `None` if the queue is empty.
	pub fn take_queue() -> Vec<TargetedMsg> {
		let msgs = read_queue();
		write_queue(Vec::new());
		msgs
	}
}
