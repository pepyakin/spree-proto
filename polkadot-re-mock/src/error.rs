use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
	/// A generic error coming from the interpreter.
	#[error("Interpreter error")]
	Interpreter(#[from] wasmi::Error),
	/// A generic I/O error has happened.
	#[error("I/O error")]
	Io(#[from] io::Error),
	#[error("{0}")]
	Msg(String),
}

impl From<String> for Error {
	fn from(msg: String) -> Self {
		Self::Msg(msg)
	}
}

impl wasmi::HostError for Error {}
