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

    #[error("Escrow expired (end_time {end_time:?})")]
    Expired {
        end_time: Option<u64>,
    },

    #[error("Escrow invalid parameter. treatments or period_days greater then 0. (treatments {treatments:?} period_days {period_days:?})")]
    InvalidParameter {
        treatments: Option<u64>,
        period_days: Option<u64>,
    },

    #[error("Not achievement contract.")]
    NotAchievementContract {},

    #[error("Escrow not expired")]
    NotExpired {},
}
