//! Module that implements wasm environment of a parachain validation function.
//!
//! Since we are only concerned about simulation of SPREE, this is pretty stripped down. We do not
//! bother ourselves here about concerns like `head_data` or anything similar.
//!
//! OTOH, we provide the `call_spree` function which allows parachain wasm code to call in to a
//! given SPREE module.

use crate::{error::Error, spree::SpreeModule, util};
use wasmi::{
	Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryRef, ModuleImportResolver,
	ModuleInstance, ModuleRef, RuntimeArgs, RuntimeValue, Signature, Trap, ValueType,
};

/// Indexes for the host functions.
///
/// This module is exclusively for constant definitions.
mod fn_index {
	pub const CALL_SPREE: usize = 0;
}

/// Resolver for the functions that might be imported by a wasm blob.
///
/// Currently, it only resolves functions from the host.
struct ParachainImportResolver;

impl<'a> ModuleImportResolver for ParachainImportResolver {
	fn resolve_func(
		&self,
		field_name: &str,
		req_signature: &Signature,
	) -> Result<FuncRef, wasmi::Error> {
		use self::ValueType::*;

		let func_ref = match field_name {
			"call_spree" => FuncInstance::alloc_host(
				Signature::new(&[I32, I32, I32, I32][..], None),
				fn_index::CALL_SPREE,
			),
			_ => {
				return Err(wasmi::Error::Function(format!(
					"host module doesn't export function with name {}",
					field_name
				)));
			}
		};
		if req_signature != func_ref.signature() {
			return Err(wasmi::Error::Function(format!(
				"wrong signature requested {}",
				field_name
			)));
		}
		Ok(func_ref)
	}
}

/// Host environment for parachain wasm.
///
/// It serves calls from the wasm instance to the host.
///
/// This is a short-lived structure and it only lives during the call into wasm.
struct ParachainHostEnv<'a, 'b> {
	/// Linear memory of the calling wasm. Used for access the wasm's linear memory during
	/// the host calls.
	linear_memory: MemoryRef,
	/// Registered instances for this parachain.
	spree_modules: &'b mut [&'a mut SpreeModule],
}

impl<'a, 'b> Externals for ParachainHostEnv<'a, 'b> {
	fn invoke_index(
		&mut self,
		index: usize,
		args: RuntimeArgs,
	) -> Result<Option<RuntimeValue>, Trap> {
		match index {
			fn_index::CALL_SPREE => {
				let handle: u32 = args.nth(0);
				let time_slice: u32 = args.nth(1);
				let blob_ptr: u32 = args.nth(2);
				let blob_len: u32 = args.nth(3);

				// Copy the specified blob.
				let blob_buf = self
					.linear_memory
					.get(blob_ptr, blob_len as usize)
					.map_err(Error::from)?;

				// Call in to the specified module passing the blob into it.
				let spree_module = self
					.spree_modules
					.get_mut(handle as usize)
					.ok_or_else(|| Error::Msg(format!("handle `{}` doesn't exist", handle)))?;
				spree_module.invoke(time_slice, blob_buf)?;

				Ok(None)
			}
			_ => panic!("unknown function index"),
		}
	}
}

fn instantiate_parachain(parachain_binary: &str) -> Result<ModuleRef, Error> {
	let mut imports = ImportsBuilder::new();
	imports.push_resolver("env", &ParachainImportResolver);

	let module = util::load_wasm_module(parachain_binary)?;
	let instance = ModuleInstance::new(&module, &imports)?.assert_no_start();

	Ok(instance)
}

/// A function that mocks the polkadot validation function.
///
/// This takes the path to parachain validation function wasm and configuration/state of SPREE
/// modules accessible (opt-in?) by this parachain.
pub fn validate_block(
	parachain_binary: &str,
	spree_modules: &mut [&mut SpreeModule],
) -> Result<(), Error> {
	let instance = instantiate_parachain(parachain_binary)?;

	let mut env = ParachainHostEnv {
		spree_modules,
		linear_memory: instance
			.export_by_name("memory")
			.ok_or_else(|| {
				Error::from("spree module expected to have export called `memory`".to_string())
			})?
			.as_memory()
			.ok_or_else(|| {
				Error::from("spree module: `memory` should be a linear memory".to_string())
			})?
			.clone(),
	};
	instance.invoke_export("validate_block", &[], &mut env)?;

	Ok(())
}
