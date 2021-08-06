use cw_storage_plus::{Item, Map};

use crate::msg::TokenInfoMsg;
use crate::types::ContractInfo;

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");