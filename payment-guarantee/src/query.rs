use cosmwasm_std::Coin;
use cw721::NftInfoResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenPriceResponse {
    pub price: Coin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NftInfoWithPriceResponse {
    pub nft_info: NftInfoResponse,
    pub price: Coin,
}