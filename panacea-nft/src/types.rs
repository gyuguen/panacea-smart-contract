use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfo {
    pub contract: String,
    pub description: Option<String>,
    pub price: Coin,
}

impl TokenInfo {
    pub fn into_token_info_with_owner(self, sender: String) -> TokenInfoWithSender {
        TokenInfoWithSender {
            contract: self.contract,
            description: self.description,
            price: self.price,
            sender,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfoWithSender {
    pub contract: String,
    pub description: Option<String>,
    pub price: Coin,
    pub sender: String,
}