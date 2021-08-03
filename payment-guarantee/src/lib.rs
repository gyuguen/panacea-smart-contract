mod msg;
mod contract;
mod state;
mod query;
mod receiver;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);