use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

#[derive(Error, Debug, PartialEq)]
pub enum OsmosisError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] cosmwasm_std::OverflowError),

    #[error("Asset not in pool")]
    AssetNotInPool,

    #[error("Aborting swap - payout: {0} is smaller then minimal output: {1}")]
    PriceTooLowExactIn(Uint128, Uint128),

    #[error("Aborting swap - payin: {0} is bigger then maximum input: {1}")]
    PriceTooLowExactOut(Uint128, Uint128),

    /// Remove this to let the compiler find all TODOs
    #[error("Not yet implemented (TODO)")]
    Unimplemented,
}