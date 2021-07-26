use cosmwasm_std::{Addr, attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};

use crate::error::ContractError;
use crate::msg::{AchievementResponse, ExecuteMsg, InstantiateMsg, JoinerResponse, QueryMsg, TermOfPaymentsResponse};
use crate::state::{Achievement, config, config_read, State};

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        payer: info.sender,
        joiner: deps.api.addr_validate(&msg.joiner)?,
        total_amount: info.funds,
        term_of_payments: msg.term_of_payments,
        achievement: Achievement {
            treatments: 0,
            insurance_claim: false,
        },
        start_time_millis: Some(env.block.time.nanos() / 1000),
        end_time_millis: msg.end_time_millis,
    };

    if state.is_expired(&env) {
        return Err(ContractError::Expired {
            end_time: msg.end_time_millis,
        });
    }

    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut state = config_read(deps.storage).load()?;
    match msg {
        ExecuteMsg::Approve {} => try_approve(deps, &env, state, info),
        ExecuteMsg::Append { treatments } => try_append(deps, &env, &mut state, info, treatments),
        ExecuteMsg::Update { treatments, insurance_claim } => try_update(deps, &env, &mut state, info, treatments, insurance_claim),
        ExecuteMsg::Refund {} => try_refund(deps, &env, state),
    }
}

fn try_append(
    deps: DepsMut,
    env: &Env,
    state: &mut State,
    info: MessageInfo,
    treatments: Option<u64>,
) -> Result<Response, ContractError> {
    if info.sender != state.payer {
        return Err(ContractError::Unauthorized { msg: "Not same 'sender' and 'payer'".to_string() });
    }

    if state.is_expired(&env) {
        return Err(ContractError::Expired {
            end_time: state.end_time_millis,
        });
    }

    state.append_treatments_in_achievement(treatments);

    config(deps.storage).save(state)?;
    Ok(Response::default())
}

fn try_update(
    deps: DepsMut,
    env: &Env,
    state: &mut State,
    info: MessageInfo,
    treatments: Option<u64>,
    insurance_claim: Option<bool>,
) -> Result<Response, ContractError> {
    if info.sender != state.payer {
        return Err(ContractError::Unauthorized { msg: "Not same 'sender' and 'payer'".to_string() });
    }

    if state.is_expired(&env) {
        return Err(ContractError::Expired {
            end_time: state.end_time_millis,
        });
    }

    state.update_treatments_in_achievement(treatments, insurance_claim);

    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

fn try_approve(
    deps: DepsMut,
    env: &Env,
    mut state: State,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    if info.sender != state.joiner {
        return Err(ContractError::Unauthorized { msg: "Not same 'sender' and 'joiner'".to_string() });
    }

    if state.is_expired(&env) {
        return Err(ContractError::Expired {
            end_time: state.end_time_millis,
        });
    }

    let amount = state.update_contract_achievement_and_get_amounts(env);

    if amount.len() == 0 {
        return Err(ContractError::NotAchievementContract {});
    }

    config(deps.storage).save(&state)?;

    Ok(send_tokens(state.joiner, amount, "approve"))
}

fn try_refund(
    deps: DepsMut,
    env: &Env,
    state: State,
) -> Result<Response, ContractError> {
    // anyone can try to refund, as long as the contract is expired
    if !state.is_expired(&env) {
        return Err(ContractError::NotExpired {});
    }

    let balance = deps.querier.query_all_balances(&env.contract.address)?;
    Ok(send_tokens(state.payer, balance, "refund"))
}

fn send_tokens(to_address: Addr, amount: Vec<Coin>, action: &str) -> Response {
    let attributes = vec![attr("action", action), attr("to", to_address.clone())];

    Response {
        submessages: vec![],
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: to_address.into(),
            amount,
        })],
        data: None,
        attributes,
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TermOfPayments {} => to_binary(&query_term_of_payments(deps)?),
        QueryMsg::Achievement {} => to_binary(&query_achievement(deps)?),
        QueryMsg::Joiner {} => to_binary(&query_joiner(deps)?),
    }
}

fn query_term_of_payments(deps: Deps) -> StdResult<TermOfPaymentsResponse> {
    let state = config_read(deps.storage).load()?;
    let term_of_payments = state.term_of_payments;
    Ok(TermOfPaymentsResponse { term_of_payments })
}

fn query_achievement(deps: Deps) -> StdResult<AchievementResponse> {
    let state = config_read(deps.storage).load()?;
    let achievement = state.achievement;
    Ok(AchievementResponse { achievement })
}

fn query_joiner(deps: Deps) -> StdResult<JoinerResponse> {
    let state = config_read(deps.storage).load()?;
    let joiner = state.joiner;
    Ok(JoinerResponse { joiner })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use cosmwasm_std::{Addr, BankMsg, coin, coins, CosmosMsg, Timestamp};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use crate::contract::{execute, instantiate, try_append, try_approve, try_refund, try_update};
    use crate::error::ContractError;
    use crate::error::ContractError::Unauthorized;
    use crate::msg::{ContractContent, ExecuteMsg, InstantiateMsg, TermOfPayment};
    use crate::state::config_read;

    fn init_msg(end_time_millis: u64, term_of_payments: Vec<TermOfPayment>) -> InstantiateMsg {
        InstantiateMsg {
            joiner: String::from("joiner"),
            term_of_payments,
            end_time_millis: Some(end_time_millis),
        }
    }

    #[test]
    fn proper_initialization() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            }
        ];

        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());

        let result = instantiate(deps.as_mut(), env, info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        let state = config_read(&mut deps.storage).load().unwrap();

        assert_eq!(&info.sender, &state.payer);
        assert_eq!(&msg.joiner, &state.joiner);
        assert_eq!(&info.funds, &state.total_amount);
        assert_eq!(1, state.term_of_payments.len());
        assert_eq!(&msg.term_of_payments[0].id, &state.term_of_payments[0].id);
        assert_eq!(&msg.term_of_payments[0].contract_content.treatments, &state.term_of_payments[0].contract_content.treatments);
        assert_eq!(&msg.term_of_payments[0].contract_content.insurance_claim, &state.term_of_payments[0].contract_content.insurance_claim);
        assert_eq!(&msg.term_of_payments[0].contract_content.period_days, &state.term_of_payments[0].contract_content.period_days);
        assert_eq!(&msg.term_of_payments[0].amount, &state.term_of_payments[0].amount);
        assert_eq!(&msg.term_of_payments[0].is_payment, &state.term_of_payments[0].is_payment);
        assert_eq!(0, state.achievement.treatments);
        assert_eq!(false, state.achievement.insurance_claim);
        assert_eq!(msg.end_time_millis, state.end_time_millis);
        assert_eq!(Some(Timestamp::from_seconds(now.as_secs()).nanos() / 1000), state.start_time_millis);
    }

    #[test]
    fn execute_update() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        let update_msg = ExecuteMsg::Update {
            treatments: Some(1000),
            insurance_claim: Some(true),
        };

        let result = execute(deps.as_mut(), env, info.clone(), update_msg);
        assert_eq!(true, result.is_ok());

        let state = config_read(&mut deps.storage).load().unwrap();

        assert_eq!(&info.sender, &state.payer);
        assert_eq!(&msg.joiner, &state.joiner);
        assert_eq!(&info.funds, &state.total_amount);
        assert_eq!(1, state.term_of_payments.len());
        assert_eq!(&msg.term_of_payments[0].id, &state.term_of_payments[0].id);
        assert_eq!(&msg.term_of_payments[0].contract_content.treatments, &state.term_of_payments[0].contract_content.treatments);
        assert_eq!(&msg.term_of_payments[0].contract_content.insurance_claim, &state.term_of_payments[0].contract_content.insurance_claim);
        assert_eq!(&msg.term_of_payments[0].contract_content.period_days, &state.term_of_payments[0].contract_content.period_days);
        assert_eq!(&msg.term_of_payments[0].amount, &state.term_of_payments[0].amount);
        assert_eq!(&msg.term_of_payments[0].is_payment, &state.term_of_payments[0].is_payment);
        assert_eq!(1000, state.achievement.treatments);
        assert_eq!(true, state.achievement.insurance_claim);
        assert_eq!(msg.end_time_millis, state.end_time_millis);
        assert_eq!(Some(Timestamp::from_seconds(now.as_secs()).nanos() / 1000), state.start_time_millis);
    }

    #[test]
    fn execute_double_update() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        let update_msg1 = ExecuteMsg::Update {
            treatments: Some(1000),
            insurance_claim: None,
        };

        let update_msg2 = ExecuteMsg::Update {
            treatments: None,
            insurance_claim: Some(true),
        };

        let result = execute(deps.as_mut(), env.clone(), info.clone(), update_msg1.clone());
        assert_eq!(true, result.is_ok());
        let result = execute(deps.as_mut(), env.clone(), info.clone(), update_msg2.clone());
        assert_eq!(true, result.is_ok());

        let state = config_read(&mut deps.storage).load().unwrap();

        assert_eq!(&info.sender, &state.payer);
        assert_eq!(&msg.joiner, &state.joiner);
        assert_eq!(&info.funds, &state.total_amount);
        assert_eq!(1, state.term_of_payments.len());
        assert_eq!(&msg.term_of_payments[0].id, &state.term_of_payments[0].id);
        assert_eq!(&msg.term_of_payments[0].contract_content.treatments, &state.term_of_payments[0].contract_content.treatments);
        assert_eq!(&msg.term_of_payments[0].contract_content.insurance_claim, &state.term_of_payments[0].contract_content.insurance_claim);
        assert_eq!(&msg.term_of_payments[0].contract_content.period_days, &state.term_of_payments[0].contract_content.period_days);
        assert_eq!(&msg.term_of_payments[0].amount, &state.term_of_payments[0].amount);
        assert_eq!(&msg.term_of_payments[0].is_payment, &state.term_of_payments[0].is_payment);
        assert_eq!(1000, state.achievement.treatments);
        assert_eq!(true, state.achievement.insurance_claim);
        assert_eq!(msg.end_time_millis, state.end_time_millis);
        assert_eq!(Some(Timestamp::from_seconds(now.as_secs()).nanos() / 1000), state.start_time_millis);
    }

    #[test]
    fn execute_append() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        let append_msg = ExecuteMsg::Append {
            treatments: Some(100),
        };
        // after 5sec, up 5 height
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        execute(deps.as_mut(), env.clone(), info.clone(), append_msg.clone());

        // after 5sec, up 5 height
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        execute(deps.as_mut(), env.clone(), info.clone(), append_msg.clone());

        let appended_state = config_read(&mut deps.storage).load().unwrap();

        assert_eq!(&info.sender, &appended_state.payer);
        assert_eq!(&msg.joiner, &appended_state.joiner);
        assert_eq!(&info.funds, &appended_state.total_amount);
        assert_eq!(1, appended_state.term_of_payments.len());
        assert_eq!(&msg.term_of_payments[0].id, &appended_state.term_of_payments[0].id);
        assert_eq!(&msg.term_of_payments[0].contract_content.treatments, &appended_state.term_of_payments[0].contract_content.treatments);
        assert_eq!(&msg.term_of_payments[0].contract_content.insurance_claim, &appended_state.term_of_payments[0].contract_content.insurance_claim);
        assert_eq!(&msg.term_of_payments[0].contract_content.period_days, &appended_state.term_of_payments[0].contract_content.period_days);
        assert_eq!(&msg.term_of_payments[0].amount, &appended_state.term_of_payments[0].amount);
        assert_eq!(&msg.term_of_payments[0].is_payment, &appended_state.term_of_payments[0].is_payment);
        assert_eq!(100, appended_state.achievement.treatments);
        assert_eq!(false, appended_state.achievement.insurance_claim);
        assert_eq!(msg.end_time_millis, appended_state.end_time_millis);
        assert_eq!(Some(Timestamp::from_seconds(now.as_secs()).nanos() / 1000), appended_state.start_time_millis);
    }

    #[test]
    fn execute_approve() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let init_amount = coins(100000000000, "umed");

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: init_amount[0].clone(),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        // update achievement
        let update_msg = ExecuteMsg::Update {
            treatments: Some(100),
            insurance_claim: Some(true),
        };
        let update_result = execute(deps.as_mut(), env.clone(), info.clone(), update_msg);
        assert_eq!(true, update_result.is_ok());

        // approve
        let approve_info = mock_info("joiner", &init_amount);
        let approve_msg = ExecuteMsg::Approve {};
        let approve_result = execute(deps.as_mut(), env.clone(), approve_info.clone(), approve_msg);
        assert_eq!(true, approve_result.is_ok());
        println!("{:?}", approve_result);
        let approve_response = approve_result.unwrap();
        let messages = approve_response.messages;
        assert_eq!(1, messages.len());
        assert_eq!(messages[0], CosmosMsg::Bank(BankMsg::Send {
            to_address: "joiner".into(),
            amount: init_amount,
        }));
        let attributes = approve_response.attributes;
        assert_eq!(2, attributes.len());
        assert_eq!("action", attributes[0].key);
        assert_eq!("approve", attributes[0].value);
        assert_eq!("to", attributes[1].key);
        assert_eq!("joiner", attributes[1].value);
    }

    #[test]
    fn execute_two_contract_one_approve() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let init_amount = coins(200000000000, "umed");

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &init_amount);
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            },
            TermOfPayment {
                id: String::from("id2"),
                contract_content: ContractContent {
                    treatments: 500,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: coin(100000000000, "umed"),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 15, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        // update
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        let update_msg = ExecuteMsg::Update {
            treatments: Some(100),
            insurance_claim: Some(true),
        };
        let update_result = execute(deps.as_mut(), env.clone(), info.clone(), update_msg);
        assert_eq!(true, update_result.is_ok());

        // approve
        let approve_info = mock_info("joiner", &init_amount);
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        let approve_result = execute(deps.as_mut(), env.clone(), approve_info.clone(), ExecuteMsg::Approve {});
        println!("{:?}", approve_result);

        assert_eq!(true, approve_result.is_ok());
        let approve_response = approve_result.unwrap();
        let messages = approve_response.messages;
        assert_eq!(1, messages.len());
        assert_eq!(messages[0], CosmosMsg::Bank(BankMsg::Send {
            to_address: "joiner".into(),
            amount: coins(100000000000, "umed"),
        }));
        let attributes = approve_response.attributes;
        assert_eq!(2, attributes.len());
        assert_eq!("action", attributes[0].key);
        assert_eq!("approve", attributes[0].value);
        assert_eq!("to", attributes[1].key);
        assert_eq!("joiner", attributes[1].value);

        // check approved state
        let approved_state = config_read(&mut deps.storage).load().unwrap();
        assert_eq!(Addr::unchecked("payer"), approved_state.payer);
        assert_eq!(Addr::unchecked("joiner"), approved_state.joiner);
        assert_eq!(&init_amount, &approved_state.total_amount);
        assert_eq!(2, approved_state.term_of_payments.len());
        let payment_1 = &approved_state.term_of_payments[0];
        assert_eq!("id1", payment_1.id);
        assert_eq!(100, payment_1.contract_content.treatments);
        assert_eq!(true, payment_1.contract_content.insurance_claim);
        assert_eq!(0, payment_1.contract_content.period_days);
        assert_eq!(coin(100000000000, "umed"), payment_1.amount);
        assert_eq!(true, payment_1.is_payment);
        let payment_2 = &approved_state.term_of_payments[1];
        assert_eq!("id2", payment_2.id);
        assert_eq!(500, payment_2.contract_content.treatments);
        assert_eq!(true, payment_2.contract_content.insurance_claim);
        assert_eq!(0, payment_2.contract_content.period_days);
        assert_eq!(coin(100000000000, "umed"), payment_2.amount);
        assert_eq!(false, payment_2.is_payment);
        assert_eq!(100, approved_state.achievement.treatments);
        assert_eq!(true, approved_state.achievement.insurance_claim);
        assert_eq!(msg.end_time_millis, approved_state.end_time_millis);
        assert_eq!(Some(Timestamp::from_seconds(now.as_secs()).nanos() / 1000), approved_state.start_time_millis);
    }

    #[test]
    fn execute_approve_unauthorized() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let init_amount = coins(100000000000, "umed");

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: init_amount[0].clone(),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        // approve
        let approve_info = mock_info("payer", &init_amount);
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        let approve_result = execute(deps.as_mut(), env.clone(), approve_info, ExecuteMsg::Approve {});

        assert_eq!(true, approve_result.is_err());

        match approve_result.unwrap_err() {
            ContractError::Unauthorized { msg, .. } => assert_eq!(
                msg,
                "Not same 'sender' and 'joiner'"
            ),
            _ => panic!("Wrong error message"),
        }
    }

    #[test]
    fn execute_approve_no_achievement() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let init_amount = coins(100000000000, "umed");

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: init_amount[0].clone(),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        // approve
        let approve_info = mock_info("joiner", &init_amount);
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        let approve_result = execute(deps.as_mut(), env.clone(), approve_info, ExecuteMsg::Approve {});

        assert_eq!(true, approve_result.is_err());

        match approve_result.unwrap_err() {
            ContractError::NotAchievementContract { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn execute_refund() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let init_amount = coins(100000000000, "umed");

        let mut deps = mock_dependencies(&[]);
        let mut env = mock_env();
        env.block.height = 876;
        env.block.time = Timestamp::from_seconds(now.as_secs());
        let info = mock_info("payer", &coins(100000000000, "umed"));
        let term_of_payments = vec![
            TermOfPayment {
                id: String::from("id1"),
                contract_content: ContractContent {
                    treatments: 100,
                    insurance_claim: true,
                    period_days: 0,
                },
                amount: init_amount[0].clone(),
                is_payment: false,
            }
        ];
        let msg = init_msg(now.as_millis() as u64 + 1000 * 5, term_of_payments.clone());
        let result = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(true, result.is_ok());
        let response = result.unwrap();
        assert_eq!(0, response.messages.len());

        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);

        let refund_info = mock_info("payer", &init_amount);
        env.block.height = env.block.height.clone() + 5;
        env.block.time = env.block.time.plus_seconds(5);
        let refund_result = execute(deps.as_mut(), env.clone(), refund_info.clone(), ExecuteMsg::Refund {});
        assert_eq!(true, refund_result.is_ok());

        println!("{:?}", refund_result);
    }
}