use std::fmt;

use cosmwasm_std::{Addr, Coin, Env, Storage};
use cosmwasm_storage::{ReadonlySingleton, singleton, Singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{ContractContent, TermOfPayment};

static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub payer: Addr,
    pub joiner: Addr,
    pub total_amount: Vec<Coin>,
    pub term_of_payments: Vec<TermOfPayment>,
    pub achievement: Achievement,
    pub start_time_millis: Option<u64>,
    pub end_time_millis: Option<u64>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "payer: {} \
        joiner: {} \
        total_amount: {:?} \
        term_of_payments: {:?} \
        start_time: {} \
        end_time: {}",
               self.payer,
               self.joiner,
               self.total_amount,
               self.term_of_payments,
               self.start_time_millis.unwrap_or(0),
               self.end_time_millis.unwrap_or(0))
    }
}

impl State {
    pub fn is_expired(&self, env: &Env) -> bool {
        if let Some(end_time_millis) = self.end_time_millis {
            if env.block.time.nanos() > end_time_millis * 1000000 {
                return true;
            }
        }
        false
    }

    pub fn update_contract_achievement_and_get_amounts(&mut self, env: &Env) -> Vec<Coin> {
        let mut amount: Vec<Coin> = Vec::new();
        let achievement = &self.achievement;

        for term_of_payment in &mut self.term_of_payments {
            if term_of_payment.is_payment == true {
                continue;
            }

            if achievement.is_achievement(&term_of_payment.contract_content, self.start_time_millis, env) {
                amount.push(term_of_payment.amount.clone());
                term_of_payment.is_payment = true;
            }
        }

        amount
    }

    pub fn append_treatments_in_achievement(&mut self, treatments: Option<u64>) {
       self.achievement.treatments = self.achievement.treatments + treatments.unwrap();
    }

    pub fn update_treatments_in_achievement(&mut self, treatments: Option<u64>, insurance_claim: Option<bool>) {
        if treatments != None {
            self.achievement.treatments = treatments.unwrap();
        }
        if insurance_claim != None {
            self.achievement.insurance_claim = insurance_claim.unwrap();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Achievement {
    pub treatments: u64,
    pub insurance_claim: bool,
}

impl Achievement {
    pub fn is_achievement(&self, contract_content: &ContractContent, start_time: Option<u64>, env: &Env) -> bool {
        if contract_content.treatments > self.treatments {
            return false;
        } else if contract_content.insurance_claim == true {
            if self.insurance_claim == false {
                return false;
            }
        } else if env.block.time.nanos() < start_time.unwrap() + (contract_content.period_days * 24 * 60 * 60 * 1000) {
            return false;
        }

        return true;
    }

    pub fn update(&mut self, treatments: Option<u64>, insurance_claim: Option<bool>) {
        if treatments != None {
            self.treatments = treatments.unwrap()
        }
        if insurance_claim != None {
            self.insurance_claim = insurance_claim.unwrap()
        }
    }
}


pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}