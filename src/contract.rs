#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, BankMsg, StdError, IbcMsg, SubMsg};
use cw2::set_contract_version;
use cw_osmo_proto::osmosis::gamm::v1beta1::{ MsgSwapExactAmountIn, SwapAmountInRoute as Osmo_SwapAmountInRoute };
use cw_osmo_proto::cosmos::base::v1beta1::{ Coin as Osmo_Coin };
use cw_osmo_proto::proto_ext::MessageExt;

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
        ExecuteMsg::Swap { pool_id, token_out_denom, token_out_min_amount } => execute_swap(_env.contract.address.into(), info, pool_id, token_out_denom, token_out_min_amount),
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

pub fn execute_swap(self_address: String, info: MessageInfo, pool_id: u64, token_out_denom: String, token_out_min_amount: String) -> Result<Response, ContractError> {
    let funds = info.funds.clone().pop().unwrap();
    let coin = Osmo_Coin {
        denom: funds.denom,
        amount: funds.amount.to_string()
    };

    let mut osmo_routes: Vec<Osmo_SwapAmountInRoute> = Vec::new();
    osmo_routes.push(Osmo_SwapAmountInRoute {
        pool_id,
        token_out_denom
    });

    let msg = MsgSwapExactAmountIn {
        sender: self_address,
        routes: osmo_routes,
        token_in: Option::from(coin),
        token_out_min_amount,
    };

    let msg = msg.to_msg()?;
    let submsg = SubMsg::new(msg);

    Ok(Response::new()
        .add_attribute("method", "execute_swap")
        .add_submessage(submsg))
}
