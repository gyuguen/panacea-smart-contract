use cw_storage_plus::{Map};
use cosmwasm_std::Coin;

pub const TOKEN_PRICE: Map<String, Coin> = Map::new("token_amount");