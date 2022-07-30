use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_storage_plus::Map;
use crate::msg::ExecuteMsg;

pub const COMMANDS_STACK: Item<Vec<ExecuteMsg>> = Item::new( "commands_stack");
pub const CONTRACT_ADDRESS: Item<Addr> = Item::new( "contract_address");
// Mapping between connections and the counter on that connection.
pub const CONNECTION_COUNTS: Map<String, u32> = Map::new("connection_counts");
