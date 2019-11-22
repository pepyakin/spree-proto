//! Bindings to the SPREE host API.

use codec::Decode;
use primitives::ParaId;

mod ffi {
	use super::ParaId;

	extern "C" {
		// # DESIGN NOTE
		//
		// Scratch buffer is some buffer on the host side that holds temporary data. The need for
		// it stems from the fact that that some functions can return a byte blob of arbitrary size
		// and there is no way for the host environment to pin point a place in the instance's
		// linear memory where to write this result since theoretically the wasm module decides
		// for itself how to layout the linear memory.
		//
		// This solution was shameless stolen from the contracts module.
		//
		// There are other ways to solve this problem however, for example:
		//
		// # Tell the host how to allocate
		//
		// The wasm module can supply the allocation routines to the host and then the functions
		// that require returning arbitrary sized outputs can call them in order to allocate
		// an appropriately sized buffer. This way a write can be performed directly to the
		// allocated memory without leaving the host environment and requiring another buffer.
		//
		// The thing to watch out for is this potentially allows recursive calls to the host,
		// (i.e. a SPREE module calls `storage_read`, then it calls the SPREE module's alloc
		// function, and then that calls some other function, etc) which can potentially lead
		// to undesired effects.
		//
		// # Remove all functions with arbitrary sized outputs
		//
		// If we have luxury to set meaningful limit on the output buffer size which is small
		// enough (up to a couple wasm pages (each one being 64K)), then we can get away with
		// making all functions to take a pointer to output buffer and just write the result there,
		// possibly returning the size of written region. The SPREE module would typically reserve
		// a region for that and always use the same pointer.

		/// Returns the current size of the scratch buffer.
		pub fn scratch_buf_size() -> usize;

		/// Copy the scratch buffer into the memory of this instance.
		///
		/// Will write out the contents of the scratch buffer to the area with the size of the
		/// scratch buffer.
		pub fn scratch_buf_read(out_ptr: *const u8);

		/// Send a message blob, specified by `blob_ptr` and `blob_len` to the SPREE module's
		/// doppelganger on the opposite side of the ICMP channel specified by `para_id`.
		///
		/// Returns 0 on success or non-0 otherwise.
		pub fn send(para_id: ParaId, blob_ptr: *const u8, blob_len: usize) -> usize;

		/// Fill the scratch buffer with all inbound messages.
		///
		/// All messages are encoded as Vec<(sender: ParaId, blob: [u8])>
		pub fn poll();

		/// Reads storage by a given key.
		///
		/// The key is passed in a buffer, represented by `key_ptr` and `key_len`.
		///
		/// Returns 0 if the key found or non-zero otherwise.
		/// The result blob of the read is stored in the scratch buffer.
		pub fn storage_read(key_ptr: *const u8, key_len: usize) -> usize;

		/// Writes a storage value by a given key.
		///
		/// The key is passed in a buffer represented by `key_ptr` and `key_len` and the value
		/// is represented by `val_ptr` and `val_len`.
		pub fn storage_write(
			key_ptr: *const u8,
			key_len: usize,
			val_ptr: *const u8,
			val_len: usize,
		);
	}
}

pub fn storage_read(key: &[u8]) -> Option<Vec<u8>> {
	unsafe {
		if ffi::storage_read(key.as_ptr(), key.len()) == 0 {
			let output = scratch_buf_read();
			Some(output)
		} else {
			None
		}
	}
}

pub fn storage_write(key: &[u8], val: &[u8]) {
	unsafe {
		ffi::storage_write(key.as_ptr(), key.len(), val.as_ptr(), val.len());
	}
}

pub fn scratch_buf_read() -> Vec<u8> {
	unsafe {
		let size = ffi::scratch_buf_size();
		if size == 0 {
			return Vec::new();
		}
		let mut output = Vec::with_capacity(size);
		ffi::scratch_buf_read(output.as_mut_ptr());
		output.set_len(size);
		output
	}
}

pub fn send(recepient: ParaId, blob: &[u8]) {
	unsafe {
		ffi::send(recepient, blob.as_ptr(), blob.len());
	}
}

pub fn poll() -> Vec<(ParaId, Vec<u8>)> {
	unsafe {
		ffi::poll();

		let raw_poll_msg = scratch_buf_read();
		<Vec<(ParaId, Vec<u8>)>>::decode(&mut &raw_poll_msg[..])
			.expect("poll is guaranteed to return this type")
	}
}
