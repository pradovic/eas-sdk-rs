use crate::contracts::eas::{EASErrors, EAS as Contract};
use ethers::contract::ContractError;
use ethers::providers::Middleware;

use thiserror::Error;

mod attest;
mod revoke;
mod timestamp;

pub use attest::*;
pub use revoke::*;
pub use timestamp::*;

#[derive(Error, Debug)]
pub enum Error<M: Middleware> {
    #[error("attestation error: {0}")]
    EASError(EASErrors),

    #[error("contract error: {0}")]
    ContractError(#[from] ContractError<M>),

    #[error("unknown error: {0}")]
    Other(String),
}

pub struct EAS<M: Middleware> {
    pub contract: Contract<M>,
}

impl<M: Middleware + 'static> EAS<M> {
    pub fn new(contract: Contract<M>) -> Self {
        Self { contract }
    }

    pub fn contract(&self) -> &Contract<M> {
        &self.contract
    }
}
