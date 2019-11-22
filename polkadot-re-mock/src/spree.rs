//! Module that implements the wasm environment of a SPREE module.

use crate::error::Error;
use codec::Encode;
use std::collections::HashMap;
use wasmi::{
	Externals, FuncInstance, FuncRef, ImportsBuilder, MemoryRef, ModuleImportResolver,
	ModuleInstance, ModuleRef, RuntimeArgs, RuntimeValue, Signature, Trap, ValueType,
};

/// Indexes for the host functions.
///
/// This module is exclusively for constant definitions.
mod fn_index {

	pub const SCRATCH_BUF_SIZE: usize = 1;
	pub const SCRATCH_BUF_READ: usize = 2;
	pub const SEND: usize = 3;
	pub const POLL: usize = 4;
	pub const STORAGE_READ: usize = 5;
	pub const STORAGE_WRITE: usize = 6;
}

/// Resolver for the functions that might be imported by a wasm blob.
struct SpreeModuleImportResolver;

impl<'a> ModuleImportResolver for SpreeModuleImportResolver {
	fn resolve_func(
		&self,
		field_name: &str,
		req_signature: &Signature,
	) -> Result<FuncRef, wasmi::Error> {
		use self::ValueType::*;

		let (fn_index, param_tys, return_ty) = match field_name {
			"scratch_buf_size" => (fn_index::SCRATCH_BUF_SIZE, &[][..], Some(I32)),
			"scratch_buf_read" => (fn_index::SCRATCH_BUF_READ, &[I32][..], None),
			"send" => (fn_index::SEND, &[I32, I32, I32][..], Some(I32)),
			"poll" => (fn_index::POLL, &[][..], None),
			"storage_read" => (fn_index::STORAGE_READ, &[I32, I32][..], Some(I32)),
			"storage_write" => (fn_index::STORAGE_WRITE, &[I32, I32, I32, I32][..], None),
			_ => {
				return Err(wasmi::Error::Function(format!(
					"host module doesn't export function with name {}",
					field_name
				)));
			}
		};
		let sig = Signature::new(param_tys, return_ty);
		if req_signature != &sig {
			return Err(wasmi::Error::Function(format!(
				"wrong signature requested {}",
				field_name
			)));
		}
		let func_ref = FuncInstance::alloc_host(sig, fn_index);
		Ok(func_ref)
	}
}

struct SpreeModuleHostEnv<'a> {
	scratch_buf: Vec<u8>,
	linear_memory: MemoryRef,
	acc: &'a mut SpreeIcmpAccumulator,
	storage: &'a mut HashMap<Vec<u8>, Vec<u8>>,
}

impl<'a> SpreeModuleHostEnv<'a> {
	fn new(
		blob: Vec<u8>,
		instance: &ModuleRef,
		acc: &'a mut SpreeIcmpAccumulator,
		storage: &'a mut HashMap<Vec<u8>, Vec<u8>>,
	) -> Result<Self, Error> {
		Ok(Self {
			scratch_buf: blob,
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
			acc,
			storage,
		})
	}
}

impl<'a> Externals for SpreeModuleHostEnv<'a> {
	fn invoke_index(
		&mut self,
		index: usize,
		args: RuntimeArgs,
	) -> Result<Option<RuntimeValue>, Trap> {
		match index {
			fn_index::SCRATCH_BUF_SIZE => {
				let size = self.scratch_buf.len();
				Ok(Some(RuntimeValue::I32(size as i32)))
			}
			fn_index::SCRATCH_BUF_READ => {
				let out_ptr: u32 = args.nth(0);
				self.linear_memory
					.set(out_ptr, &self.scratch_buf[..])
					.map_err(Error::from)?;
				Ok(None)
			}
			fn_index::SEND => {
				let recepient: u32 = args.nth(0);
				let blob_ptr: u32 = args.nth(1);
				let blob_len: u32 = args.nth(2);

				let blob_buf = self
					.linear_memory
					.get(blob_ptr, blob_len as usize)
					.map_err(Error::from)?;
				match self.acc.outbound.insert(recepient, blob_buf) {
					Some(_previous) => {
						// There were an existing message, trap to signal an error.
						Ok(Some(RuntimeValue::I32(1)))
					}
					None => Ok(Some(RuntimeValue::I32(0))),
				}
			}
			fn_index::POLL => {
				self.scratch_buf = self
					.acc
					.inbound
					.iter()
					.collect::<Vec<(&u32, &Vec<u8>)>>()
					.encode();
				Ok(None)
			}
			fn_index::STORAGE_READ => {
				let key_ptr: u32 = args.nth(0);
				let key_len: u32 = args.nth(1);
				let key_buf = self
					.linear_memory
					.get(key_ptr, key_len as usize)
					.map_err(Error::from)?;
				match self.storage.get(&key_buf) {
					Some(val_ref) => {
						self.scratch_buf = val_ref.clone();
						Ok(Some(RuntimeValue::I32(0)))
					}
					None => Ok(Some(RuntimeValue::I32(1))),
				}
			}
			fn_index::STORAGE_WRITE => {
				let key_ptr: u32 = args.nth(0);
				let key_len: u32 = args.nth(1);
				let val_ptr: u32 = args.nth(2);
				let val_len: u32 = args.nth(3);

				let key_buf = self
					.linear_memory
					.get(key_ptr, key_len as usize)
					.map_err(Error::from)?;
				let val_buf = self
					.linear_memory
					.get(val_ptr, val_len as usize)
					.map_err(Error::from)?;
				self.storage.insert(key_buf, val_buf);
				Ok(None)
			}
			_ => panic!("unknown function index"),
		}
	}
}

/// Accumulator of inbound and outbound messages for a SPREE module instance.
pub struct SpreeIcmpAccumulator {
	inbound: HashMap<u32, Vec<u8>>,
	outbound: HashMap<u32, Vec<u8>>,
}

impl SpreeIcmpAccumulator {
	pub fn with_inbound_msgs(inbound: impl IntoIterator<Item = (u32, Vec<u8>)>) -> Self {
		Self {
			inbound: inbound.into_iter().collect(),
			outbound: HashMap::new(),
		}
	}
}

pub struct SpreeModule {
	wasm_path: String,
	acc: SpreeIcmpAccumulator,
	instance: Option<ModuleRef>,
	storage: HashMap<Vec<u8>, Vec<u8>>,
}

impl SpreeModule {
	pub fn new(wasm_path: impl Into<String>, acc: SpreeIcmpAccumulator) -> Self {
		Self {
			wasm_path: wasm_path.into(),
			acc,
			instance: None,
			storage: HashMap::default(),
		}
	}

	pub fn invoke(&mut self, time_slice: u32, blob: Vec<u8>) -> Result<(), Error> {
		let instance = ensure_instance(&self.wasm_path, &mut self.instance)?;

		let mut env = SpreeModuleHostEnv::new(blob, &instance, &mut self.acc, &mut self.storage)?;
		instance
			.invoke_export("handle", &[RuntimeValue::I32(time_slice as i32)], &mut env)
			.map_err(Error::from)?;
		Ok(())
	}

	pub fn outbound_messages(&self) -> &HashMap<u32, Vec<u8>> {
		&self.acc.outbound
	}
}

fn ensure_instance<'a>(
	path: &str,
	instance_cache: &'a mut Option<ModuleRef>,
) -> Result<&'a ModuleRef, Error> {
	if let Some(ref instance) = *instance_cache {
		return Ok(instance);
	}

	let mut imports = ImportsBuilder::new();
	imports.push_resolver("env", &SpreeModuleImportResolver);

	let module = crate::util::load_wasm_module(path)?;
	let instance = ModuleInstance::new(&module, &imports)?.assert_no_start();
	*instance_cache = Some(instance);

	// Option::unwrap is fine here since it is just assigned above.
	let instance_ref = instance_cache.as_ref().unwrap();
	Ok(instance_ref)
}
