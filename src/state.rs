use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use crate::msg::ExecuteMsg;

pub const COMMANDS_STACK: Item<Vec<ExecuteMsg>> = Item::new( "commands_stack");
pub const CONTRACT_ADDRESS: Item<Addr> = Item::new( "contract_address");
