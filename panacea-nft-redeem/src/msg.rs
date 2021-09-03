use cosmwasm_std::Coin;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub source_contracts: Vec<String>,
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit {},
    ReceiveNft(Cw721ReceiveMsg),
    RecoverOwner {contract: String, token_id: String},
    Refund { },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfoMsg {
    pub contract: String,
    pub description: Option<String>,
    pub price: Coin,
    pub sender: String,
}

