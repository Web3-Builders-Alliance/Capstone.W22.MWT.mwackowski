use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not Found: {val:?}")]
    NotFound {val: String},

    #[error("Invalid deposit denom: {val:?}, allowed denoms are: uosmo, usdc")]
    InvalidDepositDenom {val: String},

    #[error("Denom parsing error: {val:?}")]
    DenomParsingError {val: String},

    #[error("No ledger found for provided address: {val:?}")]
    NoLedgerFoundErr {val: String},
    
    #[error("Could not find provided etf_name: {val:?} in storage; mint contract not instantiated")]
    MintContractNotFound {val: String},

    #[error("Invalid parameters: ratios and routes should have the same length")]
    InvalidEntryParams {},

    #[error("Token {val:?} not found in pool")]
    PoolTokenNotFound {val: String},

    #[error("Sum of swap ratios needs to be equal to 100")]
    InvalidRatio {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
