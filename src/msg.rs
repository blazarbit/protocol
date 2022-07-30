use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Transfer {
        address: String,
    },
    IbcTransfer {
        channel_id: String,
        address: String,
    },
    Swap {
        pool_id: u64,
        token_out_denom: String,
        token_out_min_amount: String,
    }
}
