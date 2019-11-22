//
// # DESIGN NOTE
//
// This is the simplest way of doing this however not super performant. The reason for that is
// that `handle` acts like hourglass funnel since all calls have to be encoded
// as a buffer on the parachain side and then here they need to be decoded and dispatched.
//
// To make it more visual:
//
// 1. Parachain wasm binary calls a helper function that prepares a blob, that encodes, say
//    `(fn_index: u32, encoded_args: [u8])`.
// 2. The helper function then calls `call_spree` passing this blob.
// 3. In a spree module, the data blob will be decoded and destructed into `fn_index` and
//    `encoded_args`.
// 4. Then a dispatch will be performed of a function specified by a `fn_index`, using either
//    a direct call (using a `match` statement) or indirect call (
//
// # ADC
//
// There is an optimization, which is dubbed ADC - almost direct call, that would eliminate this
// hourglass construction. The idea is instead of having a single entrypoint `handle`, we would
// allow a SPREE module to publish arbitrary entrypoints, represented as wasm exports.
//
// Then as for accessing these entrypoints from the parachain module there are multiple options
// with various tradeoffs.
//
// ## Imports
//
// First one, is to declare them as imports, like the following:
//
// ```ignore
// (import "spree_module_1" "foo" (func (param i32 i32) (result i32))
// ```
//
// To give you a context, WebAssembly has two level namespaces. First is called "module" and the
// second "field". Both of them are arbitrary, but usually the module is used for specifying
// a module name (in 99.9999% hardcoded to "env" due to historical reasons) and the field part
// is usually used for the function name in this module. Note however that these names are totally
// arbitrary and it is up to the host environment how it would resolve them. For example, two
// modules can import an item with the same name but the host can resolve them to completely
// different functions.
//
// So here, we import a function named "foo" from a module "spree_module_1". The host might
// lookup "spree_module_1" and see if there is a SPREE module with such name is registered for this
// particular parachain and if so resolves it with a function that upon a call will instantiate
// the SPREE module, if required, and then tranfer control the requested function there
// (in this case `foo`).
//
// This is actually the lowest overhead way of doing such things. For example, in this case it
// would be a simple optimization to just call the function directly in JIT generated code.
// However, it comes with a downside that the import mechanism is a bit unflexible. Examples of
// that would be:
//
// - once a module is instantiated it will stay instantiated till the end of lifetime of
// the parachain module.
// - to reference the SPREE module only a (utf-8) string is usable.
// - wasm requires all imports to be resolved at instantiation time. Although the host might decide
//  to create stubs that trap when called, there is also a need to provide a way to check if the
//  API is present which might get hairy.
//
// ## `dlopen`/`dlsym`-like approach
//
// Another option, is to do employ `dlopen`/`dlsym` like approach. In essence, the approach is
// a more explicit version of the previous one. There would be a function available to
// the parachain module to instantiate a module and return an opaque handle to it, that would
// logically look like the following:
//
// ```ignore
// fn spree_instantiate(ref: &str) -> SpreeModuleRef;
// ```
//
// Then, `SpreeModuleRef` would have a function to resolve an exported function from the SPREE
// module instance.
//
// ```ignore
// impl SpreeModuleRef {
//     fn spree_find_export(field_name: &str) -> Option<SpreeFnRef>;
// }
// ```
//
// `SpreeFnRef` would be a ordinary function pointer, although untyped. It is up to user to
// correctly cast it to the appropriate function signature under risk of a trap.
//
// This option is a bit slower, since it has to perform some additional checks, like that the
// signature of the callee matches the expected one and that it exists, etc. Note though that by
// slower I mean orders of a hunderd cycles slower.
//
// This option provides more flexibility. For example, we can imagine API for tearing down a
// `SpreeModuleRef`, having more than one instance of the same wasm module, using various means
// to identify the functions to import, error handling is more straightforward, and so on.
//
// NB: This all assumes that we propagate panics into the host runtime. If we want to gracefully
// receive panics in the parachain then the design might look totally different.

use codec::{Decode, Encode};
use std::collections::HashMap;

mod ext;
mod storage;

use primitives::{Req, Resp, TargetedMsg, TimestampedMsg};

/// A function that handles requests coming from the SPREE runtime environment, or ultimately from
/// the parachain.
#[no_mangle]
pub extern "C" fn handle(_time_slice: usize) {
	// Execution starts with the scratch buffer filled with the input data payload passed from the
	// parachain validation function.
	let req = Req::decode(&mut &ext::scratch_buf_read()[..]).unwrap();
	match req {
		Req::Enqueue { recepient, payload } => {
			let timestamp = storage::next_timestamp();
			storage::enqueue_msg(TargetedMsg {
				recepient,
				msg: TimestampedMsg {
					at: timestamp,
					payload,
				},
			});
		}
		Req::Poll => {
			// Poll the incoming messages from our doppelgangers on the other sides.
			//
			// Each doppelganger sends one ICMP message containing a bundle of incoming timestamped
			// messages.
			let poll_result = ext::poll()
				.into_iter()
				.map(|(sender, raw_inbound_msgs)| {
					(
						sender,
						<Vec<TimestampedMsg>>::decode(&mut &raw_inbound_msgs[..]).expect(
							"doppelganger uses the same code;\
							 therefore encoding/decoding should be symmetrical;\
							 it shouldn't fail;\
							 qed",
						),
					)
				})
				.collect();
			let _ = Resp {
				inbound: poll_result,
			};
			// TODO: Return the result.
			//
			// Should be trivial to return the data via some means, e.g. scratch buffer?
		}
		Req::FanOut => {
			// Group all messages by the recepient.
			let msg_by_recepient = storage::take_queue()
				.into_iter()
				.map(|msg| (msg.recepient, msg.msg))
				.fold(HashMap::new(), |mut acc, (recepient, msg)| {
					acc.entry(recepient).or_insert_with(Vec::new).push(msg);
					acc
				});
			for (recepient, msgs) in msg_by_recepient {
				ext::send(recepient, &msgs.encode());
			}
		}
	}
}
