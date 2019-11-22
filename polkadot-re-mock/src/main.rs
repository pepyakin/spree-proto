//! Polkadot Runtime Environment mock.

use codec::Encode;
use spree_lamport_clock_primitives::TimestampedMsg;

mod error;
mod parachain;
mod spree;
mod util;

use error::Error;
use spree::{SpreeIcmpAccumulator, SpreeModule};

const PARACHAIN_WASM: &str =
	"./dummy-parachain/target/wasm32-unknown-unknown/debug/dummy_parachain.wasm";
const SPREE_LAMPORT_CLOCK_WASM: &str =
	"./spree-lamport-clock/target/wasm32-unknown-unknown/debug/spree_lamport_clock.wasm";

fn main() -> Result<(), Error> {
	// Initialize a SPREE module with the given wasm module and inbound messages.
	let mut lamport_clock = SpreeModule::new(
		SPREE_LAMPORT_CLOCK_WASM,
		SpreeIcmpAccumulator::with_inbound_msgs(vec![(
			0,
			TimestampedMsg {
				at: 0,
				payload: b"bar".to_vec(),
			}
			.encode(),
		)]),
	);

	// Call in the polkadot validation function with the given parachain wasm and given set
	// of SPREE modules.
	parachain::validate_block(PARACHAIN_WASM, &mut [&mut lamport_clock])?;

	// Verify that expected messages were sent by the SPREE module.
	assert_eq!(
		lamport_clock.outbound_messages(),
		&vec![(
			1,
			vec![TimestampedMsg {
				at: 1,
				payload: b"foo".to_vec()
			}]
			.encode()
		)]
		.into_iter()
		.collect(),
	);

	Ok(())
}
