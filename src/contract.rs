use crate::msg::{
    HandleAnswer, HandleMsg, InitMsg, QueryMsg, QueryWithPermit, ResponseStatus, ScoreResponse,
    StateResponse, StatsResponse,
};
use crate::state::{
    does_user_exist, load, may_load, read_viewing_key, save, write_viewing_key, Config, Constants,
    ReadonlyConfig, State, User, CONFIG_KEY,
};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, QueryResult, StdError, StdResult, Storage,
};
use ripemd160::{Digest};
use secret_toolkit::permit::{validate, Permission, Permit, RevokedPermits};
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
        prng_seed: sha_256(base64::encode(msg.prng_seed).as_bytes()).to_vec(),
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
        HandleMsg::Record { score, description } => try_record(deps, env, score, description),
        HandleMsg::RevokePermit { permit_name, .. } => revoke_permit(deps, env, permit_name),
        HandleMsg::WithPermit { permit, query } => permit_handle(deps, permit, query),
        HandleMsg::GenerateViewingKey { entropy, .. } => {
            try_generate_viewing_key(deps, env, entropy)
        }
    }
}

pub fn try_generate_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: String,
) -> StdResult<HandleResponse> {
    let config: State = load(&deps.storage, CONFIG_KEY)?;
    let prng_seed = config.prng_seed;

    let key = ViewingKey::new(&env, &prng_seed, (&entropy).as_ref());

    let message_sender = deps.api.canonical_address(&env.message.sender)?;

    write_viewing_key(&mut deps.storage, &message_sender, &key);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::GenerateViewingKey { key })?),
    })
}

fn permit_handle<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    permit: Permit,
    query: QueryWithPermit,
) -> StdResult<HandleResponse> {
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
                    "No permission to query score, got permissions {:?}",
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
    description: String,
) -> StdResult<HandleResponse> {
    let sender_address = deps.api.canonical_address(&env.message.sender)?;
    let user_state = does_user_exist(&deps.storage, sender_address.as_slice());
    let description = description.as_bytes();
    // create the User struct containing score  and timestamp
    let stored_score = User {
        score,
        timestamp: env.block.time,
        description: description.to_vec(),
    };

    save(&mut deps.storage, sender_address.as_slice(), &stored_score)?;

    if !user_state {
        let state: StateResponse = query_state(deps).unwrap();

        let new_state = State {
            max_size: state.max_size,
            score_count: state.score_count + 1,
            prng_seed: state.prng_seed,
        };

        save(&mut deps.storage, CONFIG_KEY, &new_state)?;
    }

    let status: String = String::from("Score recorded!");

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
    let description: String;
    let mut score: Option<u64> = None;
    let mut timestamp: Option<u64> = None;
    let sender_address = deps.api.canonical_address(address)?;
    let result: Option<User> = may_load(&deps.storage, sender_address.as_slice())
        .ok()
        .unwrap();

    match result {
        Some(stored_score) => {
            score = Some(stored_score.score);
            timestamp = Some(stored_score.timestamp);
            description = String::from_utf8(stored_score.description).unwrap();
            status = String::from("Score found.");
        }
        None => {
            status = String::from("Score not found.");
            description = String::from("N/A");
            return Ok(ScoreResponse {
                status,
                timestamp,
                score,
                description,
            });
        }
    }

    Ok(ScoreResponse {
        score,
        timestamp,
        status,
        description,
    })
}

fn query_stats<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<StatsResponse> {
    let config: State = load(&deps.storage, CONFIG_KEY)?;
    Ok(StatsResponse {
        score_count: config.score_count,
        max_size: config.max_size,
    })
}

fn query_state<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<StateResponse> {
    let config: State = load(&deps.storage, CONFIG_KEY)?;
    Ok(StateResponse {
        score_count: config.score_count,
        max_size: config.max_size,
        prng_seed: config.prng_seed,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetStats {} => to_binary(&query_stats(deps)?), // get the max_length allowed and the count
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
        _ => authenticated_queries(deps, msg),
    }
}

fn authenticated_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    let (addresses, key) = msg.get_validation_params();

    for address in addresses {
        let canonical_addr = deps.api.canonical_address(address)?;

        let expected_key = read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
        } else if key.check_viewing_key(expected_key.unwrap().as_slice()) {
            // return to_binary(&query_read(deps, &address)?)
            return match msg {
                QueryMsg::Read { address, .. } => to_binary(&query_read(deps, &address)?),
                _ => panic!("This query type does not require authentication"),
            };
        }
    }

    Err(StdError::unauthorized())
}


pub fn sha_256(data: &[u8]) -> [u8; SHA256_HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
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
        let init_msg = InitMsg {
            max_size: 10000,
            prng_seed: "this is a padding".to_string(),
        };
        let env = mock_env("creator", &coins(20, "token"));
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // WE RECORD THE SCORE
        let _env = mock_env("creator", &coins(20, "token"));
        let msg = HandleMsg::Record {
            score: 300,
            description: "This describes your score".to_string(),
        };
        let record_res = handle(&mut deps, _env, msg).unwrap();
        assert_eq!(0, record_res.messages.len());
    }

    #[test]
    fn stats_increment() {
        // First we init
        let mut deps = mock_dependencies(20, &coins(20, "token"));
        let init_msg = InitMsg {
            max_size: 10000,
            prng_seed: "this is a padding".to_string(),
        };
        let env = mock_env("creator", &coins(20, "token"));
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

       
        // WE RECORD THE SCORE
        let _env = mock_env("submitter", &coins(20, "token"));
        let msg = HandleMsg::Record {
            score: 300,
            description: "This describes your score".to_string(),
        };

        handle(&mut deps, _env, msg).unwrap();

        // WE QUERY THE STATS
        let stats_msg = QueryMsg::GetStats {};
        let stats_res = query(&deps, stats_msg).unwrap();
        let value: StatsResponse = from_binary(&stats_res).unwrap();
        assert_eq!(1, value.score_count);

        // WE RECORD THE SCORE A 2ND TIME WITH SAME USER
        let __env = mock_env("submitter", &coins(20, "token"));
        let msg = HandleMsg::Record {
            score: 200,
            description: "This describes your 2nd score".to_string(),
        };
        handle(&mut deps, __env, msg).unwrap();

        // WE QUERY THE STATS again
        let stats_msg2 = QueryMsg::GetStats {};
        let stats_res2 = query(&deps, stats_msg2).unwrap();
        let value2: StatsResponse = from_binary(&stats_res2).unwrap();
        assert_eq!(1, value2.score_count);
    }

    #[test]
    fn handle_revoke_permit() {
        // First we init
        let mut deps = mock_dependencies(45, &coins(2, "token"));
        let init_msg = InitMsg {
            max_size: 10000,
            prng_seed: "this is a padding".to_string(),
        };
        let env = mock_env(
            "secret1nl7dnjcs9w2a4mn4q43nwyptf3uyllp3xh44j0",
            &coins(20, "token"),
        );
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // WE RECORD THE SCORE
        let _env = mock_env(
            "secret1nl7dnjcs9w2a4mn4q43nwyptf3uyllp3xh44j0",
            &coins(20, "token"),
        );
        let msg = HandleMsg::Record {
            score: 300,
            description: String::from("Good job dude"),
        };
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

        assert_eq!(true, revoked_permits);
    }

    #[test]
    fn handle_viewing_key() {
        // First we init
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let init_msg = InitMsg {
            max_size: 10000,
            prng_seed: "this is a padding".to_string(),
        };
        let env = mock_env("creator", &coins(20, "token"));
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // WE RECORD THE SCORE
        let _env = mock_env("creator", &coins(20, "token"));
        let msg = HandleMsg::Record {
            score: 300,
            description: String::from("Your SCRTSibyl score is FAIR, with a total of 581 points, which qualifies you for a loan of up to $5000 USD. SCRTSibyl computed your score accounting for your Plaid diamond 12.5% apr interest credit card credit card your total current balance of $44520 and your 9 different bank accounts. An error occurred during computation of the metrics: velocity, and your score was rounded down. Try again later or log in using a different account."),
        };
        let record_res = handle(&mut deps, _env, msg).unwrap();
        assert_eq!(0, record_res.messages.len());

        // Create a viewing key
        let __env = mock_env("creator", &coins(20, "token"));
        let v_key_msg = HandleMsg::GenerateViewingKey {
            entropy: "This is a string".to_string(),
            padding: Some(String::from("Good job dude")),
        };

        handle(&mut deps, __env, v_key_msg).unwrap(); //generates key: "api_key_j0y+6OGIPoHIcEEJw3WiM2695AzuNcBu/qjDwDPdwUQ="

        // Query w Viewing key
        let query_msg = QueryMsg::Read {
            address: HumanAddr("creator".to_string()),
            key: ("api_key_j0y+6OGIPoHIcEEJw3WiM2695AzuNcBu/qjDwDPdwUQ=".to_string()),
        };

        let res = query(&deps, query_msg).unwrap();
        let value: ScoreResponse = from_binary(&res).unwrap();
        assert_eq!(300, value.score.unwrap());
        assert_eq!("Your SCRTSibyl score is FAIR, with a total of 581 points, which qualifies you for a loan of up to $5000 USD. SCRTSibyl computed your score accounting for your Plaid diamond 12.5% apr interest credit card credit card your total current balance of $44520 and your 9 different bank accounts. An error occurred during computation of the metrics: velocity, and your score was rounded down. Try again later or log in using a different account.", value.description);
    }
}
