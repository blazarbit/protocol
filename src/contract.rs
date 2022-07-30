#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, BankMsg, StdError, IbcMsg};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:blazarbit-protocol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { address } => execute_transfer(deps, info, address),
        ExecuteMsg::IbcTransfer { channel_id, address } => execute_ibc_transfer(deps, _env, info, channel_id, address),
    }
}

pub fn execute_transfer(deps: DepsMut, info: MessageInfo, addr: String) -> Result<Response, ContractError> {
    let to_addr = match deps.api.addr_validate(addr.clone().as_str()).ok() {
        Some(x) => x,
        None => return Err(ContractError::Unauthorized {}),
    };

    let sent_funds = info.funds.clone();
    let msg = BankMsg::Send {
        to_address: to_addr.into(),
        amount: sent_funds,
    };

    Ok(Response::new()
        .add_attribute("method", "execute_transfer")
        .add_message(msg))
}

pub fn execute_ibc_transfer(deps: DepsMut, env: Env, mut info: MessageInfo, channel_id: String, addr: String) -> Result<Response, ContractError> {
    // require some funds
    let amount = match info.funds.pop() {
        Some(coin) => coin,
        None => {
            return Err(ContractError::Std(StdError::generic_err(
                "you must send the coins you wish to ibc transfer",
            )))
        }
    };

    // construct a packet to send
    let msg = IbcMsg::Transfer {
        channel_id,
        to_address: addr,
        amount,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_ibc_transfer"))
}
