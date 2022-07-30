use std::convert::TryInto;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, BankMsg, StdError, IbcMsg, SubMsg, SubMsgResult, WasmMsg, Coin, Uint128, Reply, IbcTimeout};
use cosmwasm_std::OverflowOperation::Sub;
use cw2::set_contract_version;
use cw_osmo_proto::osmosis::gamm::v1beta1::{ MsgSwapExactAmountIn, SwapAmountInRoute as Osmo_SwapAmountInRoute };
use cw_osmo_proto::cosmos::base::v1beta1::{ Coin as Osmo_Coin };
use cw_osmo_proto::proto_ext::MessageExt;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension,
    MintMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, IbcExecuteMsg, InstantiateMsg};
use crate::state::{COMMANDS_STACK, CONTRACT_ADDRESS};

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
        ExecuteMsg::PurchaseNFT { owner, contract_addr, token_id, token_uri } => purchaseNft(deps, _env, info, contract_addr, token_id, token_uri, owner),
        ExecuteMsg::ContractHop { contract_addr, commands } => contract_hop(deps, info, contract_addr, commands),
        ExecuteMsg::Increment { channel } => Ok(Response::new()
            .add_attribute("method", "execute_increment")
            .add_attribute("channel", channel.clone())
            .add_message(IbcMsg::SendPacket {
                channel_id: channel,
                data: to_binary(&IbcExecuteMsg::Increment {})?,
                timeout: IbcTimeout::with_timestamp(_env.block.time.plus_seconds(300)),
            })),
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
        .add_message(msg)
    )
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
    Ok(Response::new()
        .add_attribute("method", "execute_swap")
        .add_message(msg))
}

// todo: Purchase logic implemented via nft mint just for HackAtom explanation,
//  need to change it to the real NFT purchase on market
pub fn purchaseNft(deps: DepsMut, env: Env, info: MessageInfo, contract_addr: String, token_id: String, token_uri: String, owner: String) -> Result<Response, ContractError> {
    let mint_msg = Cw721ExecuteMsg::Mint(MintMsg::<Extension> {
        token_id: token_id.to_string(),
        owner: owner.clone(),
        token_uri: token_uri.clone().into(),
        extension: Option::None
    });

    let msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&mint_msg)?,
        funds: info.funds.clone(),
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "purchaseNft"))
}

fn contract_hop(deps: DepsMut, info: MessageInfo, contract_addr: String, mut commands: Vec<ExecuteMsg>) -> Result<Response, ContractError> {
    match deps.api.addr_validate(contract_addr.as_str()).ok() {
        None => return Err(ContractError::Unauthorized {}),
        Some(addr) => CONTRACT_ADDRESS.save(deps.storage, &addr)?,
    };

    // todo: need to fix it:
    //  Execute error: Broadcasting transaction failed with code 32 (codespace: sdk). Log: account sequence mismatch, expected 20, got 19: incorrect account sequence
    // let funds: Vec<_> = info.funds.into_iter().map(|c| Coin{
    //     denom: c.denom,
    //     amount: c.amount / (Uint128::new(commands.len() as u128)),
    // }).collect();
    //
    // let messages: Vec<_> = commands.into_iter().map(|cmd| {
    //     WasmMsg::Execute {
    //         contract_addr: contract_addr.clone(),
    //         msg: to_binary(&cmd).unwrap(),
    //         funds: funds.clone(),
    //     }
    // }).collect();

    // let funds: Vec<_> = info.funds;
    // let mut funds = info.funds;


    let msgs = if let Some(command) = commands.pop() {
        let msg = match command {
            ExecuteMsg::Transfer { address } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Transfer { address }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::IbcTransfer { channel_id, address } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::IbcTransfer { channel_id, address }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::Swap { pool_id, token_out_denom, token_out_min_amount } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Swap {
                        pool_id,
                        token_out_denom,
                        token_out_min_amount
                    }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::PurchaseNFT { owner, contract_addr, token_id, token_uri } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::PurchaseNFT {
                        owner,
                        contract_addr,
                        token_id,
                        token_uri
                    }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::ContractHop { contract_addr, commands } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::ContractHop { contract_addr, commands }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::Increment { channel } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Increment { channel }).unwrap(),
                    funds: info.funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
        };
        vec![msg]
    } else { vec![] };

    COMMANDS_STACK.save(deps.storage, &commands)?;

    Ok(Response::new()
        .add_attribute("method", "contract_hop")
        .add_submessages(msgs))
}

#[entry_point]
fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id { 1 => {
        hop_reply(deps, env, msg.result)
    }
        _ => return Err(ContractError::Unauthorized {})
    }
}

pub fn hop_reply(deps: DepsMut, env: Env, msg: SubMsgResult) -> Result<Response, ContractError> {
    msg.into_result().map_err(|err| StdError::generic_err(err))?;
    let mut commands = COMMANDS_STACK.load(deps.storage)?;
    let contract_addr = CONTRACT_ADDRESS.load(deps.storage)?.into_string();

    let funds = deps.querier.query_all_balances(env.contract.address)?;
    let msgs = if let Some(command) = commands.pop() {
        let msg = match command {
            ExecuteMsg::Transfer { address } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Transfer { address }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::IbcTransfer { channel_id, address } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::IbcTransfer { channel_id, address }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::Swap { pool_id, token_out_denom, token_out_min_amount } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Swap {
                        pool_id,
                        token_out_denom,
                        token_out_min_amount
                    }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::PurchaseNFT { owner, contract_addr, token_id, token_uri } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::PurchaseNFT {
                        owner,
                        contract_addr,
                        token_id,
                        token_uri
                    }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::ContractHop { contract_addr, commands } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::ContractHop { contract_addr, commands }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
            ExecuteMsg::Increment { channel } => {
                let msg = WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&ExecuteMsg::Increment { channel }).unwrap(),
                    funds: funds.clone(),
                };
                SubMsg::reply_on_success(msg, 1)
            }
        };
        vec![msg]
    } else { vec![] };

    COMMANDS_STACK.save(deps.storage, &commands)?;
    Ok(Response::new()
        .add_attribute("method", "hop_reply").add_submessages(msgs))
}
