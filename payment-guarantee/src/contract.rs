use std::borrow::BorrowMut;

use cosmwasm_std::{attr, Binary, BlockInfo, Coin, coin, Deps, DepsMut, Empty, Env, from_binary, MessageInfo, Order, Pair, Response, StdError, StdResult, to_binary};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw721::{Cw721QueryMsg, Cw721ReceiveMsg, Expiration, NftInfoResponse, NumTokensResponse, TokensResponse};
use cw721_base::ContractError;
use cw721_base::msg::{InstantiateMsg, MinterResponse};

use crate::msg::{ExecuteMsg, MintMsg, QueryMsg};
use crate::query::{NftInfoWithPriceResponse, TokenPriceResponse};
use crate::receiver::NftInfoWithPriceMsg;
use crate::state::TOKEN_PRICE;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    cw721_base::contract::instantiate(deps, _env, _info, msg)
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
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => cw721_base::contract::execute_approve(deps, env, info, spender, token_id, expires),
        ExecuteMsg::Revoke {
            spender,
            token_id,
        } => cw721_base::contract::execute_revoke(deps, env, info, spender, token_id),
        ExecuteMsg::ApproveAll {
            operator,
            expires,
        } => cw721_base::contract::execute_approve_all(deps, env, info, operator, expires),
        ExecuteMsg::RevokeAll {
            operator
        } => cw721_base::contract::execute_revoke_all(deps, env, info, operator),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => cw721_base::contract::execute_transfer_nft(deps, env, info, recipient, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
        } => execute_send_nft(deps, env, info, contract, token_id),
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
    }
}

pub fn execute_mint(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MintMsg,
) -> Result<Response, ContractError> {
    let nft_mint_msg = cw721_base::msg::MintMsg {
        token_id: msg.token_id.clone(),
        owner: msg.owner.clone(),
        name: msg.name.clone(),
        description: msg.description.clone(),
        image: msg.image.clone(),
    };

    let mint_response = cw721_base::contract::execute_mint(deps.branch(), _env, info.clone(), nft_mint_msg.clone());

    if mint_response.is_err() {
        return mint_response;
    }

    TOKEN_PRICE.save(deps.storage, msg.token_id.clone(), &msg.price);

    Ok(Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "mint"),
            attr("minter", info.sender.clone()),
            attr("token_id", msg.token_id.clone()),
            attr("price", msg.price.clone()),
        ],
        data: None,
    })
}

pub fn execute_send_nft(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    token_id: String,
) -> Result<Response, ContractError> {
    // get nft_info_with_price
    let nft_info_with_price = query_nft_info_with_price(
        deps.as_ref(),
        env.clone(),
        token_id.clone())?;
    let nft_info = nft_info_with_price.nft_info;
    let price = nft_info_with_price.price;
    let nft_info_with_price_msg = NftInfoWithPriceMsg {
        name: nft_info.name.clone(),
        description: Some(nft_info.description.clone()),
        image: nft_info.image.clone(),
        price: price.clone(),
    };

    TOKEN_PRICE.remove(deps.storage, token_id.to_string());

    cw721_base::contract::execute_send_nft(
        deps,
        env.clone(),
        info.clone(),
        contract.clone(),
        token_id.clone(),
        Some(nft_info_with_price_msg.into_binary()?))
}

pub fn execute_receive_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    // get minter this contract
    let res: StdResult<MinterResponse> = from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Minter {})?);
    let response = res?;
    let minter = response.minter;

    // convert binary to nft_info_with_price_msg
    let nft_info_with_price_msg: NftInfoWithPriceMsg = from_binary(&msg.msg.unwrap())?;

    let mint_msg = nft_info_with_price_msg.into_mint_msg(minter, msg.token_id.clone())?;

    execute_mint(deps, env, info, mint_msg.clone())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Minter {} => cw721_base::contract::query(deps, env, cw721_base::msg::QueryMsg::Minter {}),
        QueryMsg::ContractInfo {} => cw721_base::contract::query(deps, env, cw721_base::msg::QueryMsg::ContractInfo {}),
        QueryMsg::NftInfo {
            token_id
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::NftInfo {
                token_id,
            }),
        QueryMsg::OwnerOf {
            token_id,
            include_expired
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::OwnerOf {
                token_id,
                include_expired,
            }),
        QueryMsg::AllNftInfo {
            token_id,
            include_expired,
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            }),
        QueryMsg::ApprovedForAll {
            owner,
            include_expired,
            start_after,
            limit,
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::ApprovedForAll {
                owner,
                include_expired,
                start_after,
                limit,
            }),
        QueryMsg::NumTokens {} => cw721_base::contract::query(deps, env, cw721_base::msg::QueryMsg::NumTokens {}),
        QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
        ),
        QueryMsg::AllTokens {
            start_after,
            limit,
        } => cw721_base::contract::query(
            deps,
            env,
            cw721_base::msg::QueryMsg::AllTokens {
                start_after,
                limit,
            },
        ),
        QueryMsg::NftInfoWithPrice { token_id } => to_binary(&query_nft_info_with_price(deps, env, token_id)?),
    }
}

fn query_token_price(
    deps: Deps,
    token_id: String,
) -> Coin {
    let op = TOKEN_PRICE.may_load(deps.storage, token_id).unwrap();
    op.unwrap_or(coin(0, "umed".to_string()))
}

fn query_nft_info_with_price(deps: Deps, env: Env, token_id: String) -> StdResult<NftInfoWithPriceResponse> {
    let res = cw721_base::contract::query(deps, env, cw721_base::msg::QueryMsg::NftInfo { token_id: token_id.clone() });
    if res.is_err() {
        return Err(res.unwrap_err());
    }

    let res: StdResult<NftInfoResponse> = from_binary(&res.unwrap());
    if res.is_err() {
        return Err(res.unwrap_err());
    }
    let nft_info_response = res.unwrap();

    let price = query_token_price(deps, token_id.clone());

    Ok(NftInfoWithPriceResponse {
        nft_info: nft_info_response,
        price,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{CosmosMsg, from_binary, OwnedDeps, WasmMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cw721::{Approval, ApprovedForAllResponse, OwnerOfResponse};
    use cw721_base::contract::{execute_transfer_nft, execute_revoke_all};
    use cw721_base::msg::MinterResponse;

    use crate::msg::QueryMsg;

    use super::*;

    const MINTER: &str = "medibloc";
    const CONTRACT_NAME: &str = "payment guarantee";
    const SYMBOL: &str = "MED_PG";

    fn setup_contract(deps: DepsMut, env: Env, info: MessageInfo) {
        let msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            minter: MINTER.to_string(),
        };

        let res = instantiate(deps, env.clone(), info, msg);
        assert_eq!(true, res.is_ok());
        assert_eq!(0, res.unwrap().messages.len());
    }

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);
        let env = mock_env();
        setup_contract(deps.as_mut(), env.clone(), info.clone());

        let res = query(deps.as_ref(), env.clone(), QueryMsg::Minter {});
        assert_eq!(true, res.is_ok());
        let res: MinterResponse = from_binary(&res.unwrap()).unwrap();
        assert_eq!(MINTER, res.minter);
    }

    #[test]
    fn minting() {
        let mut deps = mock_dependencies(&[]);
        let instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let env = mock_env();
        setup_contract(deps.as_mut(), env.clone(), instant_info.clone());

        let mint_msg = MintMsg {
            token_id: "join_reward_1".to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };

        // error
        let random_mint_info = mock_info("random", &[]);
        let res = execute_mint(deps.as_mut(), mock_env(), random_mint_info, mint_msg.clone());
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});

        // allow
        let mint_info = mock_info(MINTER, &[]);
        let result = execute_mint(deps.as_mut(), env.clone(), instant_info.clone(), mint_msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(vec![
            attr("action", "mint"),
            attr("minter", instant_info.sender.clone()),
            attr("token_id", mint_msg.token_id.clone()),
            attr("price", mint_msg.price.clone()),
        ],
                   response.attributes);

        // check token price
        let response = query_nft_info_with_price(deps.as_ref(), env.clone(), mint_msg.token_id.clone()).unwrap();
        assert_eq!(mint_msg.name, response.nft_info.name);
        assert_eq!(mint_msg.description, Some(response.nft_info.description));
        assert_eq!(mint_msg.image, response.nft_info.image);
        assert_eq!(mint_msg.price, response.price);

        // check token count
        let res = query(deps.as_ref(), env.clone(), QueryMsg::NumTokens {});
        assert!(res.is_ok());
        let res: StdResult<NumTokensResponse> = from_binary(&res.unwrap());
        assert!(res.is_ok());
        assert_eq!(1, res.unwrap().count);

        // mint 2
        let mint_info = mock_info(MINTER, &[]);
        let mint_msg2 = MintMsg {
            token_id: "join_reward_2".to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };
        let result = execute_mint(deps.as_mut(), env.clone(), instant_info.clone(), mint_msg2.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(vec![
            attr("action", "mint"),
            attr("minter", instant_info.sender.clone()),
            attr("token_id", mint_msg2.token_id.clone()),
            attr("price", mint_msg2.price.clone()),
        ],
                   response.attributes);

        // check token2
        let response = query_nft_info_with_price(deps.as_ref(), env.clone(), mint_msg2.token_id.clone()).unwrap();
        assert_eq!(mint_msg2.name, response.nft_info.name);
        assert_eq!(mint_msg2.description, Some(response.nft_info.description));
        assert_eq!(mint_msg2.image, response.nft_info.image);
        assert_eq!(mint_msg2.price, response.price);

        // check token count
        let res = query(deps.as_ref(), env.clone(), QueryMsg::NumTokens {});
        assert!(res.is_ok());
        let res: StdResult<NumTokensResponse> = from_binary(&res.unwrap());
        assert!(res.is_ok());
        assert_eq!(2, res.unwrap().count);
    }

    fn init_mint(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>, env: Env, mint_msg: MintMsg) {
        let mint_info = mock_info(MINTER, &[]);
        let result = execute_mint(deps.as_mut(), env.clone(), mint_info, mint_msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            vec![
                attr("action", "mint"),
                attr("minter", MINTER),
                attr("token_id", mint_msg.token_id.clone()),
                attr("price", mint_msg.price.clone()),
            ],
            response.attributes);

        let response = query_nft_info_with_price(deps.as_ref(), env.clone(), mint_msg.token_id.clone()).unwrap();
        assert_eq!(mint_msg.name, response.nft_info.name);
        assert_eq!(mint_msg.description, Some(response.nft_info.description));
        assert_eq!(mint_msg.image, response.nft_info.image);
        assert_eq!(mint_msg.price, response.price);
    }

    #[test]
    fn approve_revoke() {
        let mut deps = mock_dependencies(&[]);
        let instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let env = mock_env();
        setup_contract(deps.as_mut(), env.clone(), instant_info.clone());

        let token_id = "join_reward_1";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };

        init_mint(&mut deps, env.clone(), mint_msg.clone());

        // Give random transferring power
        let approve_info = mock_info(mint_msg.owner.as_str(), &[]);
        let spender = "random";
        let approve_msg = ExecuteMsg::Approve {
            spender: spender.to_string(),
            token_id: token_id.to_string(),
            expires: None,
        };
        let res = execute(deps.as_mut(), env.clone(), approve_info.clone(), approve_msg);
        assert_eq!(
            res.unwrap(),
            Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "approve"),
                    attr("sender", approve_info.sender.as_str()),
                    attr("spender", spender),
                    attr("token_id", token_id.to_string()),
                ],
                data: None,
            }
        );
        // check add approvals
        let res = query(deps.as_ref(), env.clone(), QueryMsg::OwnerOf { token_id: mint_msg.token_id.clone(), include_expired: None });
        assert!(res.is_ok());
        let res: StdResult<OwnerOfResponse> = from_binary(&res.unwrap());
        let response = res.unwrap();
        assert_eq!(mint_msg.owner.as_str(), response.owner);
        assert_eq!(
            vec![Approval {
                spender: spender.to_string(),
                expires: Expiration::Never {},
            }],
            response.approvals);

        let revoke_info = mock_info(mint_msg.owner.as_str(), &[]);
        let revoke_msg = ExecuteMsg::Revoke {
            spender: spender.to_string(),
            token_id: token_id.to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), revoke_info.clone(), revoke_msg.clone());
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "revoke"),
                    attr("sender", revoke_info.sender.as_str()),
                    attr("spender", spender.to_string()),
                    attr("token_id", token_id.clone()),
                ],
                data: None,
            });

        // check remove approval
        let res = query(deps.as_ref(), env.clone(), QueryMsg::OwnerOf { token_id: token_id.to_string(), include_expired: None });
        assert!(res.is_ok());
        let res: StdResult<OwnerOfResponse> = from_binary(&res.unwrap());
        let response = res.unwrap();
        assert_eq!(mint_msg.owner.as_str(), response.owner);
        assert_eq!(
            0,
            response.approvals.len());
    }

    #[test]
    fn approve_all_revoke_all() {
        let mut deps = mock_dependencies(&[]);
        let instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let env = mock_env();
        // instantiate
        setup_contract(deps.as_mut(), env.clone(), instant_info.clone());

        let token_id = "join_reward_1";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };

        // mint nft
        init_mint(&mut deps, env.clone(), mint_msg.clone());

        // approve all
        let nft_owner = mint_msg.owner;
        let approve_all_info = mock_info(nft_owner.as_str(), &[]);
        let operator = "operator";
        let approve_all_msg = ExecuteMsg::ApproveAll {
            operator: operator.to_string(),
            expires: None,
        };
        let res = execute(deps.as_mut(), env.clone(), approve_all_info.clone(), approve_all_msg);
        assert!(res.is_ok());
        assert_eq!(
            Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "approve_all"),
                    attr("sender", approve_all_info.sender.as_str()),
                    attr("operator", operator),
                ],
                data: None,
            },
            res.unwrap());

        // get approvals and check count 1
        let res = query(deps.as_ref(), env.clone(), QueryMsg::ApprovedForAll {
            owner: nft_owner.clone(),
            include_expired: None,
            start_after: None,
            limit: None,
        });
        assert!(res.is_ok());
        let res: StdResult<ApprovedForAllResponse> = from_binary(&res.unwrap());
        assert!(res.is_ok());
        let operators = res.unwrap().operators;
        assert_eq!(1, operators.len());
        assert_eq!(Some(&Approval {
            spender: operator.to_string(),
            expires: Expiration::Never {},
        }), operators.get(0));

        // revoke all
        let revoke_all_info = mock_info(nft_owner.as_str(), &[]);
        let revoke_all_msg = ExecuteMsg::RevokeAll {
            operator: operator.to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), revoke_all_info.clone(), revoke_all_msg.clone());
        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "revoke_all"),
                    attr("sender", revoke_all_info.sender.as_str()),
                    attr("operator", operator),
                ],
                data: None,
            }
        );

        // get approvals and check not exist
        let res = query(deps.as_ref(), env.clone(), QueryMsg::ApprovedForAll {
            owner: nft_owner.clone(),
            include_expired: None,
            start_after: None,
            limit: None,
        });
        assert!(res.is_ok());
        let res: StdResult<ApprovedForAllResponse> = from_binary(&res.unwrap());
        assert!(res.is_ok());
        let operators = res.unwrap().operators;
        assert_eq!(0, operators.len());
    }

    #[test]
    fn transfer_nft() {
        let mut deps = mock_dependencies(&[]);
        let instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let env = mock_env();
        // instantiate
        setup_contract(deps.as_mut(), env.clone(), instant_info.clone());

        let token_id = "join_reward_1";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };

        // mint nft
        init_mint(&mut deps, env.clone(), mint_msg.clone());

        let transfer_info = mock_info(mint_msg.owner.as_str(), &[]);
        let recipient = "user2";
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: token_id.to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), transfer_info.clone(), transfer_msg.clone());
        assert!(res.is_ok());
        assert_eq!(
            Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "transfer_nft"),
                    attr("sender", transfer_info.sender.clone()),
                    attr("recipient", recipient),
                    attr("token_id", mint_msg.token_id.clone()),
                ],
                data: None,
            },
            res.unwrap()
        );

        let res = query(deps.as_ref(), env.clone(), QueryMsg::OwnerOf { token_id: token_id.to_string(), include_expired: None });
        assert!(res.is_ok());
        let response: StdResult<OwnerOfResponse> = from_binary(&res.unwrap());
        assert_eq!(recipient.to_string(), response.unwrap().owner);
    }

    #[test]
    fn send_nft() {
        // create first contract
        let mut deps = mock_dependencies(&[]);
        let instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let env = mock_env();
        // instantiate
        setup_contract(deps.as_mut(), env.clone(), instant_info.clone());

        let token_id = "join_reward_1";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "user1".to_string(),
            name: "Join rewards".to_string(),
            description: Some("Token paid upon initial subscription".to_string()),
            image: None,
            price: coin(100000000000, "umed"),
        };

        // mint nft
        init_mint(&mut deps, env.clone(), mint_msg.clone());

        let send_info = mock_info(mint_msg.owner.as_str(), &[]);

        let res = execute_send_nft(deps.as_mut(), env.clone(), send_info, "contract2".to_string(), token_id.to_string());
        let response = res.unwrap();
        let message = response.messages.get(0);



        let mut receive_msg = None;
        if let CosmosMsg::Wasm(wasm) = message.unwrap() {
            if let WasmMsg::Execute { contract_addr, msg, send } = wasm {
                println!("{:?} {:?} {:?}", contract_addr, msg, send);
                if let ExecuteMsg::ReceiveNft(m) = from_binary(&msg).unwrap() {
                    receive_msg = Some(m);
                }
            }
        }

        if receive_msg == None {
            panic!("receive_msg not set.")
        }

        let receive_msg = receive_msg.unwrap();
        let send_token_id = receive_msg.token_id.clone();
        let sender = receive_msg.sender.clone();
        let info_msg:NftInfoWithPriceMsg = from_binary(&receive_msg.clone().msg.unwrap()).unwrap();

        assert_eq!(mint_msg.name, info_msg.name);
        assert_eq!(mint_msg.description, info_msg.description);
        assert_eq!(mint_msg.image, info_msg.image);
        assert_eq!(mint_msg.price, info_msg.price);


        let mut new_deps = mock_dependencies(&[]);
        let new_env = mock_env();
        let new_instant_info = mock_info(MINTER, &[coin(100000000000, "umed")]);
        let receive_info = mock_info("user2", &[]);

        setup_contract(new_deps.as_mut(), new_env.clone(), new_instant_info.clone());

        let res = execute_receive_nft(new_deps.as_mut(), new_env.clone(), receive_info.clone(), receive_msg.clone());
        println!("{:?}", res)

    }
}