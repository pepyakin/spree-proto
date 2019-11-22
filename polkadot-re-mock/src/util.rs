use crate::error::Error;
use wasmi::Module;

pub fn load_wasm_module(path: &str) -> Result<Module, Error> {
	use std::{fs::File, io::prelude::*};
	let mut file = File::open(path)?;
	let mut wasm_buf = Vec::new();
	file.read_to_end(&mut wasm_buf)?;

	let module = wasmi::Module::from_buffer(&wasm_buf)?;
	Ok(module)
}
