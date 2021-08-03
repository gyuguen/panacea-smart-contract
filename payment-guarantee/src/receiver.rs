use cosmwasm_std::{Binary, Coin, StdResult, to_binary};
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::msg::MintMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftInfoWithPriceMsg {
    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents
    pub description: Option<String>,
    /// A URI pointing to an image representing the asset
    pub image: Option<String>,
    pub price: Coin,
}

impl NftInfoWithPriceMsg {
    pub fn into_binary(self) -> StdResult<Binary> {
        to_binary(&self)
    }

    pub fn into_cw721_receive_msg(self, sender: String, token_id: String) -> StdResult<Cw721ReceiveMsg> {
        let msg = self.into_binary()?;
        Ok(Cw721ReceiveMsg {
            sender: sender.to_string(),
            token_id: token_id.clone(),
            msg: Some(msg),
        })
    }

    pub fn into_mint_msg(self, owner:String, token_id:String) -> StdResult<MintMsg> {
        Ok(MintMsg {
            token_id,
            owner: owner.to_string(),
            name: self.name.to_string(),
            description: self.description.clone(),
            image: self.image.clone(),
            price: self.price.clone(),
        })
    }
}