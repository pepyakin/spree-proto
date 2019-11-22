//! A striped-down version of a parachain validation function.

use codec::Encode;
use spree_lamport_clock_primitives::Req;

mod ext;

fn call_lamport_clock(req: Req) {
	ext::call_spree(0, 1337, &req.encode());
}

#[no_mangle]
pub extern "C" fn validate_block() {
	call_lamport_clock(Req::Poll);
	call_lamport_clock(Req::Enqueue {
		recepient: 1,
		payload: b"foo".to_vec(),
	});
	call_lamport_clock(Req::FanOut);
}
