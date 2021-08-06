use cosmwasm_std::Coin;
use cw721::{Cw721ReceiveMsg, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,
}

impl InstantiateMsg {
    pub fn into_cw721_instantiate_msg(self) -> cw721_base::msg::InstantiateMsg {
        cw721_base::msg::InstantiateMsg {
            name: self.name,
            symbol: self.symbol,
            minter: self.minter,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft { recipient: String, token_id: String },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke { spender: String, token_id: String },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },

    /// Mint a new NFT, can only be called by the contract minter
    Mint(MintMsg),
}

impl ExecuteMsg {
    pub fn into_cw721_execute_msg(self) -> cw721_base::msg::ExecuteMsg {
        match self {
            ExecuteMsg::Mint(msg) => cw721_base::msg::ExecuteMsg::Mint(msg.into_cw721_mint_msg()),
            ExecuteMsg::Approve { spender, token_id, expires } => cw721_base::msg::ExecuteMsg::Approve { spender, token_id, expires },
            ExecuteMsg::Revoke { spender, token_id } => cw721_base::msg::ExecuteMsg::Revoke { spender, token_id },
            ExecuteMsg::ApproveAll { operator, expires } => cw721_base::msg::ExecuteMsg::ApproveAll { operator, expires },
            ExecuteMsg::RevokeAll { operator } => cw721_base::msg::ExecuteMsg::RevokeAll { operator },
            ExecuteMsg::TransferNft { recipient, token_id } => cw721_base::msg::ExecuteMsg::TransferNft { recipient, token_id },
            ExecuteMsg::SendNft { contract, token_id } => cw721_base::msg::ExecuteMsg::SendNft { contract, token_id, msg: None },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg {
    pub token_id: Option<String>,
    /// The owner of the newly minter NFT
    pub owner: String,
    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents (may be empty)
    pub description: Option<String>,
    /// A URI pointing to an image representing the asset
    pub image: Option<String>,

    pub price: Coin,
}

impl MintMsg {
    pub fn into_cw721_mint_msg(self) -> cw721_base::msg::MintMsg {
        cw721_base::msg::MintMsg {
            token_id: self.token_id.unwrap_or_default(),
            owner: self.owner.to_string(),
            name: self.name.to_string(),
            description: self.description.clone(),
            image: self.image.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReceiverExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
}


/*use cosmwasm_std::{Binary, Coin};
use cw721::{Expiration, Cw721ReceiveMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft { recipient: String, token_id: String },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke { spender: String, token_id: String },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },

    /// Mint a new NFT, can only be called by the contract minter
    Mint(MintMsg),
}

impl ExecuteMsg {
    pub fn into_cw721_execute_msg(self) -> cw721_base::msg::ExecuteMsg {
        match self {
            ExecuteMsg::Mint(msg) => cw721_base::msg::ExecuteMsg::Mint(msg.into_cw721_mint_msg()),
            ExecuteMsg::Approve { spender, token_id, expires } => cw721_base::msg::ExecuteMsg::Approve { spender, token_id, expires },
            ExecuteMsg::Revoke { spender, token_id } => cw721_base::msg::ExecuteMsg::Revoke { spender, token_id },
            ExecuteMsg::ApproveAll { operator, expires } => cw721_base::msg::ExecuteMsg::ApproveAll { operator, expires },
            ExecuteMsg::RevokeAll { operator } => cw721_base::msg::ExecuteMsg::RevokeAll { operator },
            ExecuteMsg::TransferNft { recipient, token_id } => cw721_base::msg::ExecuteMsg::TransferNft { recipient, token_id },
            ExecuteMsg::SendNft { contract, token_id } => cw721_base::msg::ExecuteMsg::SendNft { contract, token_id, msg: None },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg {
    pub token_id: Option<String>,
    /// The owner of the newly minter NFT
    pub owner: String,
    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents (may be empty)
    pub description: Option<String>,
    /// A URI pointing to an image representing the asset
    pub image: Option<String>,

    pub price: Coin,
}

impl MintMsg {
    pub fn into_cw721_mint_msg(self) -> cw721_base::msg::MintMsg {
        cw721_base::msg::MintMsg {
            token_id: self.token_id.unwrap_or_default(),
            owner: self.owner.to_string(),
            name: self.name.to_string(),
            description: self.description.clone(),
            image: self.image.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReceiverExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
}*/