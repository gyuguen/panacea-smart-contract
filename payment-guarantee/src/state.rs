use cw_storage_plus::{Item, Map};

use crate::query::ContractInfoResponse;
use crate::types::TokenOwnerInfo;

pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("contract_info");
pub const TOKEN_OWNER_INFO: Map<(String, String), TokenOwnerInfo> = Map::new("token_info");