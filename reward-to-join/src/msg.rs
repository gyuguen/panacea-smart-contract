use cosmwasm_std::{Addr, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::state::Achievement;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub joiner: String,
    pub term_of_payments: Vec<TermOfPayment>,
    pub end_time_millis: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractContent {
    pub treatments: u64,
    pub insurance_claim: bool,
    pub period_days: u64,
}

impl fmt::Display for ContractContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "treatments: {}, insurance_claim: {}, period_days: {}",
               self.treatments, self.insurance_claim, self.period_days)

    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TermOfPayment {
    pub id: String,
    pub contract_content: ContractContent,
    pub amount: Coin,
    pub is_payment: bool,
}

impl fmt::Display for TermOfPayment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id: {}, contract_content: {}, amount:{}, is_payment:{}",
               self.id, self.contract_content, self.amount, self.is_payment)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Update {
        treatments: Option<u64>,
        insurance_claim: Option<bool>,
    },
    Append {
        treatments: Option<u64>,
    },
    Approve {},
    Refund {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TermOfPayments {},
    Achievement {},
    Joiner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TermOfPaymentsResponse {
    pub term_of_payments: Vec<TermOfPayment>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AchievementResponse {
    pub achievement: Achievement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct JoinerResponse {
    pub joiner: Addr,
}

