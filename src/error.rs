//! Crate errors

use std::fmt::{Debug, Display};

pub enum BindgenError {
    Syn(syn::Error),
    IO(std::io::Error),
    ParseErr {
        at: String,
        message: String,
    },
    /// No main module present
    MainModules {
        /// If many is true, that basically means there are more than 1 modules, while the opposite means
        /// there are zero.
        many: bool
    },
}

impl BindgenError {
    fn format_err(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syn(err) => write!(f, "{err}"),
    
            Self::IO(err) => write!(f, "{err}"),
    
            Self::ParseErr {at, message} => {
                write!(f, "Caught a parsing error on {at}: {message}")
            },
    
            Self::MainModules {many} => {
                let msg = if *many {
                    "Found multiple main modules, while only 1 can be present at the same time"
                } else {
                    "No main module found, can't construct a declaration file"
                };
                write!(f, "{msg}")
            }
        }
    }
}

impl std::error::Error for BindgenError {}

impl Debug for BindgenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format_err(f)
    }
}

impl Display for BindgenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format_err(f)
    }
}

impl From<syn::Error> for BindgenError {
    fn from(value: syn::Error) -> Self {
        Self::Syn(value)
    }
}

impl From<std::io::Error> for BindgenError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}