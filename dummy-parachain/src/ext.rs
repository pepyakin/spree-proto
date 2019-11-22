//! Bindings to the polkadot runtime interface.

pub type SpreeHandle = usize;

mod ffi {
	use super::SpreeHandle;

	extern "C" {
		/// A low-level API to call a SPREE module specified by spree handle provided by the host
		/// environment.
		///
		/// An argument can be passed as a byte blob, represented by `blob_ptr` and `blob_len`.
		pub fn call_spree(
			handle: SpreeHandle,
			time_slice: usize,
			blob_ptr: *const u8,
			blob_len: usize,
		);
	}
}

/// Call into a SPREE module specified by a given `handle`.
pub fn call_spree(handle: SpreeHandle, time_slice: usize, blob: &[u8]) {
	unsafe {
		ffi::call_spree(handle, time_slice, blob.as_ptr(), blob.len());
	}
}
