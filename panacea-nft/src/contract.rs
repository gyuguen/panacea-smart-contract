use cosmwasm_std::{Binary, Deps, DepsMut, Env, from_binary, MessageInfo, Response, StdResult, to_binary, to_vec};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw721::{NumTokensResponse, OwnerOfResponse};
use cw721_base::ContractError;
use cw721_base::msg::QueryMsg;

use crate::{ExecuteMsg, InstantiateMsg, MintMsg, TokenInfo};

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
        ExecuteMsg::SendNft { contract, token_id } => execute_send_nft(deps, env, info, contract, token_id),
        _ => cw721_base::contract::execute(deps, env, info, msg.into_cw721_execute_msg()),
    }
}

fn execute_mint(deps: DepsMut, env: Env, info: MessageInfo, mut msg: MintMsg) -> Result<Response, ContractError> {
    let contract_info: cw721::ContractInfoResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::ContractInfo {})?)?;
    let symbol = contract_info.symbol;
    let num_token_res: NumTokensResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::NumTokens {})?)?;
    let next_index = num_token_res.count + 1;

    let token_id: String = [symbol, next_index.to_string()].join(".");

    let token_info = TokenInfo {
        price: msg.price.clone(),
    };
    msg.description = Some(String::from_utf8(to_vec(&token_info).unwrap()).unwrap());

    cw721_base::contract::execute_mint(deps, env, info, msg.into_cw721_mint_msg(token_id))
}

fn execute_send_nft(deps: DepsMut, env: Env, info: MessageInfo, contract: String, token_id: String) -> Result<Response, ContractError> {
    let owner_of: OwnerOfResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    })?)?;

    cw721_base::contract::execute_send_nft(deps, env, info, contract, token_id, Some(to_binary(&owner_of)?))
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
    use cw721::{Approval, Cw721ExecuteMsg, Expiration};
    use cw721_base::msg::ExecuteMsg::Approve;
    use cw721_base::msg::InstantiateMsg;

    use crate::msg::ReceiverExecuteMsg;
    use crate::types::TokenInfo;

    use super::*;

    const MINTER: &str = "minter";
    const CONTRACT_NAME: &str = "Magic Power";
    const SYMBOL: &str = "N_MED";

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
        assert_eq!([SYMBOL, "1"].join("."), token_id);

        let nft_info: NftInfoResponse = from_binary(&cw721_base::contract::query(deps.as_ref(), env.clone(), QueryMsg::NftInfo { token_id }).unwrap()).unwrap();

        let token_info: TokenInfo = from_slice(nft_info.description.as_bytes()).unwrap();
        assert_eq!(coin(1000000, "umed"), token_info.price);
    }

    #[test]
    fn test_execute_send() {
        let mut deps = mock_dependencies(&[]);
        setup_contract(deps.as_mut());
        let env = mock_env();

        let mint_msg = MintMsg {
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

        let approve_info = mock_info(mint_msg.owner.as_str(), &[]);
        let spender = "spender";
        let res = cw721_base::contract::execute_approve(deps.as_mut(), env.clone(), approve_info.clone(), spender.to_string(), token_id.to_string(), None);
        assert!(res.is_ok());

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
                    let owner_of: OwnerOfResponse = from_binary(&receiver_msg.msg.unwrap()).unwrap();
                    assert_eq!(mint_msg.owner, owner_of.owner);
                    assert_eq!(
                        vec![Approval {
                            spender: spender.to_string(),
                            expires: Expiration::Never {},
                        }],
                        owner_of.approvals);
                }
            }
        }
    }
}