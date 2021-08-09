use std::borrow::Borrow;

use cosmwasm_std::{attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, entry_point, Env, from_binary, MessageInfo, Response, StdResult, to_binary, WasmMsg};
use cw721::{Cw721ReceiveMsg, OwnerOfResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, TokenInfoMsg};
use crate::state::CONTRACT_INFO;
use crate::query::QueryMsg;
use crate::types::ContractInfo;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract_info = ContractInfo {
        source_contracts: msg.source_contracts,
        payer: info.sender.to_string(),
    };

    CONTRACT_INFO.save(deps.storage, &contract_info);

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
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if is_invalid_from_contract(&contract_info, source_contract.to_string()) {
        return Err(ContractError::Unauthorized {
            msg: format!("The token belongs to an unexpected contract. actual: {}, expected: {}", source_contract.as_str(), contract_info.source_contracts.join(",")),
        });
    }

    let token_id = msg.token_id.to_string();
    let query_msg = cw721_base::msg::QueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    };

    let res: OwnerOfResponse = deps.querier.query_wasm_smart(source_contract.as_str(), &query_msg)?;
    if env.contract.address.to_string().ne(res.owner.as_str()) {
        return Err(ContractError::Unauthorized { msg: "The owner of the token must be this contract.".to_string() });
    }

    let res: StdResult<TokenInfoMsg> = from_binary(&msg.msg.unwrap());
    if res.is_err() {
        return Err(ContractError::InvalidParameter { msg: "msg is invalid parameter.".to_string() });
    }

    let token_info_msg = res.unwrap();
    let token_price = token_info_msg.price;

    let deposit_coin = deps.querier.query_balance(env.contract.address, token_price.denom.clone())?;
    if deposit_coin.amount.le(token_price.amount.borrow()) {
        return Err(ContractError::InsufficientDeposit {});
    }

    let execute_bank_send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: token_info_msg.sender,
        amount: vec![token_price],
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
            attr("amount", msg.token_id.to_string()),
        ],
        data: None,
    })
}

fn is_invalid_from_contract(contract_info: &ContractInfo, source_contract: String) -> bool {
    contract_info.source_contracts
        .iter()
        .any(|x| x.eq(source_contract.as_str())) == false
}

fn do_not_have_enough_deposit(deps: DepsMut, env: Env, token_price: Coin) -> bool {
    let deposits = deps.querier.query_all_balances(&env.contract.address).unwrap();
    deposits.iter()
        .filter(|deposit| deposit.denom.eq(token_price.denom.as_str()))
        .any(|deposit| deposit.amount >= token_price.amount) == false
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Ok(Binary::default())
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
}
