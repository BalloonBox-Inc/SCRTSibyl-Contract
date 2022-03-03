use crate::msg::{
    HandleAnswer, HandleMsg, InitMsg, QueryMsg, QueryWithPermit, ResponseStatus, ScoreResponse,
    StatsResponse,
};
use crate::state::{
    does_user_exist, load, may_load, save, Config, Constants, ReadonlyConfig, State, User,
    CONFIG_KEY,
};
use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, QueryResult, StdError, StdResult, Storage,
};
use ripemd160::{Digest, Ripemd160};
use secp256k1::Secp256k1;
use secret_toolkit::permit::{Permission, Permit, RevokedPermits, SignedPermit};
use sha2::Sha256;

pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";
pub const SHA256_HASH_SIZE: usize = 32;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let max_size = match valid_max_size(msg.max_size) {
        Some(v) => v,
        None => {
            return Err(StdError::generic_err(
                "Invalid max_size. Must be in the range of 1..65535.",
            ))
        }
    };

    let state = State {
        max_size,
        score_count: 0_u64,
    };

    save(&mut deps.storage, CONFIG_KEY, &state)?;

    let mut config = Config::from_storage(&mut deps.storage);
    config.set_constants(&Constants {
        contract_address: env.contract.address,
    })?;

    Ok(InitResponse::default())
}

// limit the max message size to values in 1..65535
fn valid_max_size(val: u16) -> Option<u16> {
    if val < 1 {
        None
    } else {
        Some(val)
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Record { score } => try_record(deps, env, score),
        HandleMsg::RevokePermit { permit_name, .. } => revoke_permit(deps, env, permit_name),
        HandleMsg::WithPermit { permit, query } => permit_handle(deps, permit, query, env),
    }
}

fn permit_handle<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    permit: Permit,
    query: QueryWithPermit,
    env: Env,
) -> StdResult<HandleResponse> {
    // Validate permit content
    let token_address = ReadonlyConfig::from_storage(&deps.storage)
        .constants()?
        .contract_address;

    if env.message.sender.to_string() != permit.params.permit_name {
        return Err(StdError::generic_err(
            "Permission for this sender has not been authorized.".to_string(),
        ));
    }

    let account = validate(deps, PREFIX_REVOKED_PERMITS, &permit, token_address)?;
    // Permit validated! We can now execute the query.

    match query {
        QueryWithPermit::Balance {} => {
            if !permit.check_permission(&Permission::Balance) {
                return Err(StdError::generic_err(format!(
                    "No permission to query balance (score), got permissions {:?}",
                    permit.params.permissions
                )));
            }
        }
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::PermitHandle {
            data: query_read(deps, &account),
        })?),
    })
}

fn revoke_permit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    permit_name: String,
) -> StdResult<HandleResponse> {
    RevokedPermits::revoke_permit(
        &mut deps.storage,
        PREFIX_REVOKED_PERMITS,
        &env.message.sender,
        &permit_name,
    );

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RevokePermit {
            status: ResponseStatus::Success,
        })?),
    })
}

pub fn try_record<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    score: u64,
) -> StdResult<HandleResponse> {
    let status: String;
    let sender_address = deps.api.canonical_address(&env.message.sender)?;
    let user_state = does_user_exist(&deps.storage, &sender_address.as_slice().to_vec());

    // create the User struct containing score  and timestamp
    let stored_score = User {
        score,
        timestamp: env.block.time,
    };

    save(
        &mut deps.storage,
        &sender_address.as_slice().to_vec(),
        &stored_score,
    )?;

    if user_state {
        let state = query_stats(deps).unwrap();

        let new_state = State {
            max_size: state.max_size,
            score_count: state.score_count + 1,
        };

        save(&mut deps.storage, CONFIG_KEY, &new_state)?;
    }

    status = String::from("Score recorded!");

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Record { status })?),
    })
}

fn query_read<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
) -> StdResult<ScoreResponse> {
    let status: String;
    let mut score: Option<u64> = None;
    let mut timestamp: Option<u64> = None;
    let sender_address = deps.api.canonical_address(address)?;
    let result: Option<User> = may_load(&deps.storage, &sender_address.as_slice().to_vec())
        .ok()
        .unwrap();

    match result {
        Some(stored_score) => {
            score = Some(stored_score.score);
            timestamp = Some(stored_score.timestamp);
            status = String::from("Score found.");
        }
        None => {
            status = String::from("Reminder not found.");
            return Ok(ScoreResponse {
                status,
                timestamp,
                score,
            });
        }
    }

    Ok(ScoreResponse {
        score,
        timestamp,
        status,
    })
}

fn query_stats<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<StatsResponse> {
    let config: State = load(&deps.storage, CONFIG_KEY)?;
    Ok(StatsResponse {
        score_count: config.score_count,
        max_size: config.max_size,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetStats {} => to_binary(&query_stats(deps)?), // get the max_length allowed and the count
        QueryMsg::GetScore { address } => to_binary(&query_read(deps, &address)?),
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
    }
}

pub fn pubkey_to_account(pubkey: &Binary) -> CanonicalAddr {
    let mut hasher = Ripemd160::new();
    hasher.update(sha_256(&pubkey.0));
    CanonicalAddr(Binary(hasher.finalize().to_vec()))
}

pub fn sha_256(data: &[u8]) -> [u8; SHA256_HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
}

pub fn validate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    storage_prefix: &str,
    permit: &Permit,
    current_token_address: HumanAddr,
) -> StdResult<HumanAddr> {
    if !permit.check_token(&current_token_address) {
        return Err(StdError::generic_err(format!(
            "Permit doesn't apply to token {:?}, allowed tokens: {:?}",
            current_token_address.as_str(),
            permit
                .params
                .allowed_tokens
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<&str>>()
        )));
    }

    // Derive account from pubkey
    let pubkey = &permit.signature.pub_key.value;
    let account = deps.api.human_address(&pubkey_to_account(pubkey))?;

    // Validate permit_name
    let permit_name = &permit.params.permit_name;
    let is_permit_revoked =
        RevokedPermits::is_permit_revoked(&deps.storage, storage_prefix, &account, permit_name);
    if is_permit_revoked {
        return Err(StdError::generic_err(format!(
            "Permit {:?} was revoked by account {:?}",
            permit_name,
            account.as_str()
        )));
    }

    // // Validate signature, reference: https://github.com/enigmampc/SecretNetwork/blob/f591ed0cb3af28608df3bf19d6cfb733cca48100/cosmwasm/packages/wasmi-runtime/src/crypto/secp256k1.rs#L49-L82
    let signed_bytes = to_binary(&SignedPermit::from_params(&permit.params))?;
    let signed_bytes_hash = sha_256(signed_bytes.as_slice());
    let secp256k1_msg = secp256k1::Message::from_slice(&signed_bytes_hash).map_err(|err| {
        StdError::generic_err(format!(
            "Failed to create a secp256k1 message from signed_bytes: {:?}",
            err
        ))
    })?;

    let secp256k1_verifier = Secp256k1::verification_only();

    let secp256k1_signature =
        secp256k1::ecdsa::Signature::from_compact(&permit.signature.signature.0)
            .map_err(|err| StdError::generic_err(format!("Malformed signature: {:?}", err)))?;
    let secp256k1_pubkey = secp256k1::PublicKey::from_slice(pubkey.0.as_slice())
        .map_err(|err| StdError::generic_err(format!("Malformed pubkey: {:?}", err)))?;

    secp256k1_verifier
        .verify_ecdsa(&secp256k1_msg, &secp256k1_signature, &secp256k1_pubkey)
        .map_err(|err| {
            StdError::generic_err(format!(
                "Failed to verify signatures for the given permit: {:?}",
                err
            ))
        })?;

    Ok(account)
}

fn permit_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    permit: Permit,
    query: QueryWithPermit,
    // env: Env
) -> Result<Binary, StdError> {
    // Validate permit content
    let token_address = ReadonlyConfig::from_storage(&deps.storage)
        .constants()?
        .contract_address;

    let account = validate(deps, PREFIX_REVOKED_PERMITS, &permit, token_address)?;

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::Balance {} => {
            if !permit.check_permission(&Permission::Balance) {
                return Err(StdError::generic_err(format!(
                    "No permission to query balance (score), got permissions {:?}",
                    permit.params.permissions
                )));
            }

            to_binary(&query_read(deps, &account))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, ReadonlyStorage};

    #[test]
    fn init_recore_query() {
        // First we init
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let init_msg = InitMsg { max_size: 10000 };
        let env = mock_env("creator", &coins(20, "token"));
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // WE RECORD THE SCORE
        let _env = mock_env("creator", &coins(20, "token"));
        let msg = HandleMsg::Record { score: 300 };
        let record_res = handle(&mut deps, _env, msg).unwrap();
        assert_eq!(0, record_res.messages.len());

        // NEXT WE QUERY THE SCORE
        let query_env = mock_env("creator", &coins(20, "token"));
        let query_msg = QueryMsg::GetScore {
            address: query_env.message.sender,
        };

        let res = query(&deps, query_msg).unwrap();
        let value: ScoreResponse = from_binary(&res).unwrap();

        assert_eq!(300, value.score.unwrap());

        // Query the stats
        let res = query(&deps, QueryMsg::GetStats {}).unwrap();
        let value: StatsResponse = from_binary(&res).unwrap();
        assert_eq!(10000, value.max_size);
    }

    #[test]
    fn handle_revoke_permit() {
        // First we init
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let init_msg = InitMsg { max_size: 10000 };
        let env = mock_env("creator", &coins(20, "token"));
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // WE RECORD THE SCORE
        let _env = mock_env("creator", &coins(20, "token"));
        let msg = HandleMsg::Record { score: 300 };
        let record_res = handle(&mut deps, _env, msg).unwrap();
        assert_eq!(0, record_res.messages.len());

        // Revoke a permission
        let __env = mock_env("creator", &coins(20, "token"));
        let revoke_msg = HandleMsg::RevokePermit {
            permit_name: String::from("test"),
            padding: None,
        };

        let record_revoke = handle(&mut deps, __env, revoke_msg).unwrap();

        assert_eq!(0, record_revoke.messages.len());

        // Check if permit is revoked
        let storage_key = PREFIX_REVOKED_PERMITS.to_string() + "creator" + "test";

        let revoked_permits = deps.storage.get(storage_key.as_bytes()).is_some();
        println!("Revoked_permits: {:?}", revoked_permits);

        assert_eq!(true, revoked_permits);
    }
}
