use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized. {msg:?}")]
    Unauthorized {
        msg: String,
    },

    #[error("InvalidParameter. {msg:?}")]
    InvalidParameter {
        msg: String,
    },

    #[error("InsufficientDeposit")]
    InsufficientDeposit {},
}