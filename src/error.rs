//! Crate errors

use std::fmt::{Debug, Display};

#[cfg(feature="bindgen")]
pub enum Error {
    Syn(syn::Error),
    IO(std::io::Error),
    ParseErr {
        message: String,
    },
    /// No main module present
    MainModules {
        /// If many is true, that basically means there are more than 1 modules, while the opposite means
        /// there are zero.
        many: bool,
    },
    Unimplemented {
        message: String,
    },
}

#[cfg(feature="bindgen")]
impl Error {
    fn format_err(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syn(err) => write!(f, "{err}"),

            Self::IO(err) => write!(f, "{err}"),

            Self::ParseErr { message } => {
                write!(f, "Got a parsing error: {message}")
            }

            Self::MainModules { many } => {
                let msg = if *many {
                    "Found multiple main modules, while only 1 can be present at the same time"
                } else {
                    "No main module found, can't construct a declaration file"
                };
                write!(f, "{msg}")
            }

            Self::Unimplemented { message } => {
                write!(f, "{message}")
            }
        }
    }
}

#[cfg(feature="bindgen")]
impl std::error::Error for Error {}

#[cfg(feature="bindgen")]
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format_err(f)
    }
}

#[cfg(feature="bindgen")]
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format_err(f)
    }
}

#[cfg(feature="bindgen")]
impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Self::Syn(value)
    }
}

#[cfg(feature="bindgen")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}