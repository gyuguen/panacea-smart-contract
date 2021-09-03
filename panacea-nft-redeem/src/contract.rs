use std::borrow::Borrow;

use cosmwasm_std::{attr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, entry_point, Env, from_binary, from_slice, MessageInfo, Response, StdResult, to_binary, WasmMsg};
use cw721::{AllNftInfoResponse, Cw721ReceiveMsg, OwnerOfResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{ContractInfoResponse, QueryMsg};
use crate::state::{CONTRACT_INFO, TOKEN_OWNER_INFO};
use crate::types::{TokenInfo, TokenOwnerInfo};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract_info = ContractInfoResponse {
        source_contracts: msg.source_contracts,
        payer: info.sender.to_string(),
    };

    let res = CONTRACT_INFO.save(deps.storage, &contract_info);
    if res.is_err() {
        return Err(ContractError::Std(res.unwrap_err()))
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute_deposit(deps, env, info),
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
        ExecuteMsg::RecoverOwner { contract, token_id } => execute_recover_owner(deps, env, info, contract, token_id),
        ExecuteMsg::Refund {} => execute_refund(deps, env, info),
    }
}

pub fn execute_deposit(
    mut _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::InvalidParameter { msg: "amount is empty.".to_string() });
    }

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "deposit"),
            attr("sender", info.sender),
        ],
        data: None,
    })
}

pub fn execute_receive_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let source_contract = info.sender.clone();
    let token_id = msg.token_id.to_string();
    let owner_of: OwnerOfResponse = from_binary(&msg.msg.unwrap_or_default())?;

    let token_owner_info: TokenOwnerInfo = TokenOwnerInfo {
        sender: msg.sender.to_string(),
        owner_of,
    };

    let res = TOKEN_OWNER_INFO.save(deps.storage, (source_contract.to_string(), token_id.to_string()), &token_owner_info);
    if res.is_err() {
        return Err(ContractError::Std(res.unwrap_err()))
    }

    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if is_invalid_from_contract(&contract_info, source_contract.to_string()) {
        return Err(ContractError::Unauthorized {
            msg: format!("The token belongs to an unexpected contract. actual: {}, expected: {}", source_contract.as_str(), contract_info.source_contracts.join(",")),
        });
    }

    let query_msg = cw721_base::msg::QueryMsg::AllNftInfo {
        token_id: token_id.to_string(),
        include_expired: None,
    };

    let all_nft_info: AllNftInfoResponse = deps.querier.query_wasm_smart(source_contract.as_str(), &query_msg)?;
    if env.contract.address.to_string().ne(all_nft_info.access.owner.as_str()) {
        return Err(ContractError::Unauthorized { msg: "The owner of the token must be this contract.".to_string() });
    }

    let token_info: TokenInfo = from_slice(all_nft_info.info.description.as_bytes()).unwrap();
    let token_price = token_info.price;

    let deposit_coin = deps.querier.query_balance(env.contract.address, token_price.denom.clone())?;
    if deposit_coin.amount.lt(token_price.amount.borrow()) {
        return Err(ContractError::InsufficientDeposit {});
    }

    let execute_bank_send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: msg.sender.to_string(),
        amount: vec![token_price.clone()],
    });
    let transfer_msg = cw721_base::msg::ExecuteMsg::TransferNft {
        recipient: contract_info.payer,
        token_id: token_id.to_string(),
    };

    let execute_wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: source_contract.to_string(),
        msg: to_binary(&transfer_msg)?,
        send: vec![],
    });

    Ok(Response {
        submessages: vec![],
        messages: vec![execute_bank_send_msg, execute_wasm_msg],
        attributes: vec![
            attr("action", "receive_nft"),
            attr("sender", msg.sender.to_string()),
            attr("sender_contract", info.sender.to_string()),
            attr("token_id", msg.token_id.to_string()),
            attr("price", token_price),
        ],
        data: None,
    })
}

fn is_invalid_from_contract(contract_info: &ContractInfoResponse, source_contract: String) -> bool {
    contract_info.source_contracts
        .iter()
        .any(|x| x.eq(source_contract.as_str())) == false
}

pub fn execute_recover_owner(deps: DepsMut,
                             _env: Env,
                             _info: MessageInfo,
                             contract: String,
                             token_id: String) -> Result<Response, ContractError> {
    let res = TOKEN_OWNER_INFO.load(deps.storage, (contract.to_string(), token_id.to_string()));
    if res.is_err() {
        return Err(ContractError::Std(res.unwrap_err()));
    }
    let token_owner_info = res.unwrap();
    let sender = token_owner_info.sender;

    let transfer_msg = cw721_base::msg::ExecuteMsg::TransferNft {
        recipient: sender.to_string(),
        token_id: token_id.to_string(),
    };

    let execute_wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&transfer_msg)?,
        send: vec![],
    });


    return Ok(Response {
        submessages: vec![],
        messages: vec![execute_wasm_msg],
        attributes: vec![
            attr("action", "recover_owner"),
            attr("sender", sender.to_string()),
            attr("sender_contract", contract.to_string()),
            attr("token_id", token_id.to_string()),
        ],
        data: None,
    });
}

pub fn execute_refund(deps: DepsMut,
                      env: Env,
                      info: MessageInfo) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let payer = contract_info.payer;
    if info.sender.as_str().ne(payer.as_str()) {
        return Err(ContractError::UnmatchedPayer {});
    }

    let balances = deps.querier.query_all_balances(env.contract.address)?;

    let execute_bank_send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: balances.clone(),
    });

    return Ok(Response {
        submessages: vec![],
        messages: vec![execute_bank_send_msg],
        attributes: vec![
            attr("action", "refund"),
            attr("sender_contract", info.sender.to_string()),
            attr("refund", balances.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(", ")),
        ],
        data: None,
    });
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    CONTRACT_INFO.load(deps.storage)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_vm::testing::{mock_env, mock_info};

    use crate::msg::TokenInfoMsg;

    use super::*;

    #[test]
    fn proper_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let msg = InstantiateMsg {
            source_contracts: vec!["contract1".to_string()],
        };

        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(res.is_ok());
    }

    #[test]
    fn receive_nft() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            source_contracts: vec!["contract1".to_string()],
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(res.is_ok());

        let receive_info = mock_info("contract1", &[]);
        let owner_of_msg = OwnerOfResponse {
            owner: "sender".to_string(),
            approvals: vec![],
        };

        let receive_msg = Cw721ReceiveMsg {
            sender: "sender".to_string(),
            token_id: "token1".to_string(),
            msg: Some(to_binary(&owner_of_msg).unwrap()),
        };

        // Unfortunately, Mock, who checks Wasm, is not yet implemented.
        // So this test always fails.
        // I think it will be supported later.
        let res = execute_receive_nft(deps.as_mut(), env.clone(), receive_info.clone(), receive_msg);
        println!("{:?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn receive_nft_empty_msg() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            source_contracts: vec!["contract1".to_string()],
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(res.is_ok());

        let receive_info = mock_info("contract1", &[]);
        let token_info_msg = TokenInfoMsg {
            contract: "contract2".to_string(),
            description: Some("No description".to_string()),
            price: coin(1000000, "umed"),
            sender: "sender".to_string(),
        };

        let receive_msg = Cw721ReceiveMsg {
            sender: "sender".to_string(),
            token_id: "token1".to_string(),
            msg: Some(to_binary(&token_info_msg).unwrap()),
        };

        // Unfortunately, Mock, who checks Wasm, is not yet implemented.
        // So this test always fails.
        // I think it will be supported later.
        let res = execute_receive_nft(deps.as_mut(), env.clone(), receive_info.clone(), receive_msg);
        println!("{:?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn recover_owner() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let contract = "contract1";
        let sender = "sender";
        let refund_token_id = "token1";

        let msg = InstantiateMsg {
            source_contracts: vec![contract.to_string()],
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(res.is_ok());

        let receive_info = mock_info(contract, &[]);

        let owner_of_msg = OwnerOfResponse {
            owner: sender.to_string(),
            approvals: vec![],
        };

        let receive_msg = Cw721ReceiveMsg {
            sender: sender.to_string(),
            token_id: refund_token_id.to_string(),
            msg: Some(to_binary(&owner_of_msg).unwrap()),
        };

        // Unfortunately, Mock, who checks Wasm, is not yet implemented.
        // So this test always fails.
        // I think it will be supported later.
        let res = execute_receive_nft(deps.as_mut(), env.clone(), receive_info.clone(), receive_msg);
        assert!(res.is_err());

        let res = execute_recover_owner(deps.as_mut(), env.clone(), info.clone(), contract.to_string(), refund_token_id.to_string());
        let response = res.unwrap();
        let cosmos_msg = response.messages[0].clone();

        if let CosmosMsg::Wasm(wasm_msg) = cosmos_msg {
            if let WasmMsg::Execute { contract_addr, msg, send } = wasm_msg {
                assert_eq!(contract, contract_addr);

                if let cw721_base::msg::ExecuteMsg::TransferNft { recipient, token_id } = from_binary(&msg).unwrap() {
                    assert_eq!(sender, recipient);
                    assert_eq!(token_id, refund_token_id);
                }
            }
        }

        assert_eq!("action", response.attributes[0].key);
        assert_eq!("recover_owner", response.attributes[0].value);
        assert_eq!("sender", response.attributes[1].key);
        assert_eq!(sender.to_string(), response.attributes[1].value);
        assert_eq!("sender_contract", response.attributes[2].key);
        assert_eq!(contract.to_string(), response.attributes[2].value);
        assert_eq!("token_id", response.attributes[3].key);
        assert_eq!(refund_token_id.to_string(), response.attributes[3].value);
    }

    #[test]
    fn refund() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            source_contracts: vec!["contract1".to_string()],
        };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(res.is_ok());

        let res = execute_refund(deps.as_mut(), env, info);
        println!("{:?}", res);
        assert!(res.is_ok());

        let response = res.unwrap();
        let cosmos_msg = response.messages[0].clone();

        if let CosmosMsg::Bank(BankMsg::Send {to_address, amount}) = cosmos_msg {
            assert_eq!("creator", to_address);
        }
        assert_eq!("action", response.attributes[0].key);
        assert_eq!("refund", response.attributes[0].value);
        assert_eq!("sender_contract", response.attributes[1].key);
        assert_eq!("creator", response.attributes[1].value);
        assert_eq!("refund", response.attributes[2].key);
    }
}
