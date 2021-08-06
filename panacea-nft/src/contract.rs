use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, from_binary, MessageInfo, Response, StdError, StdResult, to_binary, to_vec, from_slice};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw721::NftInfoResponse;
use cw721_base::ContractError;
use cw721_base::msg::QueryMsg;
use cw721_base::msg::QueryMsg::NftInfo;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
use crate::types::TokenInfo;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    cw721_base::contract::instantiate(deps, env, info, msg.into_cw721_instantiate_msg())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint(msg) => execute_mint(deps, env, info, msg),
        ExecuteMsg::Approve { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
        ExecuteMsg::Revoke { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
        ExecuteMsg::ApproveAll { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
        ExecuteMsg::RevokeAll { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
        ExecuteMsg::TransferNft { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
        ExecuteMsg::SendNft { .. } => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
    }
}

fn execute_mint(deps: DepsMut, env: Env, info: MessageInfo, mut msg: MintMsg) -> Result<Response, ContractError> {
    let contract_info: cw721::ContractInfoResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::ContractInfo {})?)?;
    let symbol = contract_info.symbol;

    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    let token_id: String = [symbol, rand_string].join(".");
    msg.token_id = Some(token_id.to_string());

    let token_info = TokenInfo {
        contract: env.contract.address.to_string(),
        description: msg.description.clone(),
        price: msg.price.clone(),
    };
    msg.description = Some(String::from_utf8(to_vec(&token_info).unwrap()).unwrap());

    cw721_base::contract::execute_mint(deps, env, info, msg.into_cw721_mint_msg())
}

fn execute_send_nft(deps: DepsMut, env: Env, info: MessageInfo, contract: String, token_id: String) -> Result<Response, ContractError> {
    let nft_info: NftInfoResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), NftInfo { token_id: token_id.to_string() })?)?;

    let token_info:TokenInfo = from_slice(nft_info.description.as_bytes()).unwrap();


    let token_info_with_sender = token_info.into_token_info_with_owner(info.sender.to_string());
    cw721_base::contract::execute_send_nft(deps, env, info, contract, token_id, Some(to_binary(&token_info_with_sender)?))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    cw721_base::contract::query(deps, env, msg)
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use cosmwasm_std::{attr, coin, CosmosMsg, DepsMut, from_slice, to_vec, WasmMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw721::Cw721ExecuteMsg;
    use cw721_base::msg::InstantiateMsg;

    use crate::msg::ReceiverExecuteMsg;
    use crate::types::{TokenInfo, TokenInfoWithSender};

    use super::*;

    const MINTER: &str = "minter";
    const CONTRACT_NAME: &str = "Magic Power";
    const SYMBOL: &str = "NFT_MED";

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            minter: String::from(MINTER),
        };
        let info = mock_info("creator", &[]);
        let res = cw721_base::contract::instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_mint() {
        let mut deps = mock_dependencies(&[]);
        setup_contract(deps.as_mut());
        let env = mock_env();

        let mint_msg = MintMsg {
            token_id: None,
            owner: MINTER.to_string(),
            name: "nft_med_1".to_string(),
            description: Some("No description".to_string()),
            image: None,
            price: coin(1000000, "umed"),
        };

        let info = mock_info(MINTER, &[]);
        let res = execute_mint(deps.as_mut(), env.clone(), info, mint_msg);
        assert!(res.is_ok());

        let attributes = res.unwrap().attributes;
        let attr1 = attributes.get(0).unwrap();
        let attr2 = attributes.get(1).unwrap();
        let attr3 = attributes.get(2).unwrap();
        let token_id = attr3.value.clone();
        assert_eq!(&attr("action", "mint"), attr1);
        assert_eq!(&attr("minter", MINTER), attr2);
        assert_eq!(attr3.key, "token_id");
        assert_eq!(token_id.len(), 28);

        let nft_info: NftInfoResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::NftInfo { token_id }).unwrap()).unwrap();

        let token_info: TokenInfo = from_slice(nft_info.description.as_bytes()).unwrap();
        assert_eq!("cosmos2contract", token_info.contract);
        assert_eq!(Some("No description".to_string()), token_info.description);
        assert_eq!(coin(1000000, "umed"), token_info.price);
    }

    #[test]
    fn test_execute_send() {
        let mut deps = mock_dependencies(&[]);
        setup_contract(deps.as_mut());
        let env = mock_env();

        let mint_msg = MintMsg {
            token_id: None,
            owner: "minter2".to_string(),
            name: "nft_med_1".to_string(),
            description: Some("No description".to_string()),
            image: None,
            price: coin(1000000, "umed"),
        };

        let info = mock_info(MINTER, &[]);
        let res = execute_mint(deps.as_mut(), env.clone(), info, mint_msg.clone());
        assert!(res.is_ok());

        let token_id = res.unwrap().attributes.get(2).unwrap().value.to_string();

        let send_info = mock_info(mint_msg.owner.as_str(), &[]);
        let send_contract = "payment_guarantee".to_string();
        let res = execute_send_nft(deps.as_mut(), env.clone(), send_info.clone(), send_contract.to_string(), token_id.to_string());
        assert!(res.is_ok());

        // if let CosmosMsg::Wasm(WasmMsg::Execute (msg) = res.unwrap().messages[0].clone())

        let cosmos_msg = res.unwrap().messages[0].clone();
        if let CosmosMsg::Wasm(wasm_msg) = cosmos_msg {
            if let WasmMsg::Execute { contract_addr, msg, send } = wasm_msg {
                assert_eq!(send_contract, contract_addr);

                let receiver_execute_msg: ReceiverExecuteMsg = from_binary(&msg).unwrap();
                if let ReceiverExecuteMsg::ReceiveNft(receiver_msg) = receiver_execute_msg {
                    assert_eq!(mint_msg.owner, receiver_msg.sender);
                    assert_eq!(token_id.as_str(), receiver_msg.token_id);
                    let token_info: TokenInfoWithSender = from_binary(&receiver_msg.msg.unwrap()).unwrap();
                    assert_eq!("cosmos2contract", token_info.contract);
                    assert_eq!(Some("No description".to_string()), token_info.description);
                    assert_eq!(coin(1000000, "umed"), token_info.price);
                    assert_eq!(send_info.sender, token_info.sender);
                }
            }
        }
    }
}