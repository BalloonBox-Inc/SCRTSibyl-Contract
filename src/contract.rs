use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, HumanAddr, CanonicalAddr, QueryResult
};
use std::convert::TryFrom;
use secret_toolkit::permit::{ Permit, RevokedPermits, SignedPermit}; 
use crate::msg::{ScoreResponse, QueryWithPermit, HandleMsg, InitMsg, QueryMsg, HandleAnswer, StatsResponse, };
use crate::state::{config, Constants, Config,  save, may_load, State, CONFIG_KEY, load, ReadonlyConfig};
use secp256k1::Secp256k1;
use sha2::{ Sha256};
use ripemd160::{Digest, Ripemd160};

pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";
pub const SHA256_HASH_SIZE: usize = 32;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    
    let max_size = match valid_max_size(msg.max_size) {
        Some(v) => v,
        None => return Err(StdError::generic_err("Invalid max_size. Must be in the range of 1..65535."))
    };

    let state = State {
        max_size,
        score_count: 0_u64,
    };

    // config(&mut deps.storage).save(&state)?;
    // debug_print!("Contract was initialized by {}", env.message.sender);
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
        u16::try_from(val).ok()
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
   
    match msg {
        // HandleMsg::Increment {} => try_increment(deps, env),
        // HandleMsg::Reset { count } => try_reset(deps, env, count),
        HandleMsg::Record { score } => try_record(deps, env, score),
    }
}

pub fn try_record<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    score: u64,
) -> StdResult<HandleResponse> {
    let status: String;
    let sender_address = deps.api.canonical_address(&env.message.sender)?;

    save(&mut deps.storage, &sender_address.as_slice().to_vec(), &score)?;

    status = String::from("Score recorded!");

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Record {
            status,
        })?),
    })
}


fn query_read<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
) -> StdResult<ScoreResponse> {
    // let status: String;
    // let mut score: Option<u64> = None;
    // let mut timestamp: Option<u64> = None;

    let sender_address = deps.api.canonical_address(&address)?;


    // read the reminder from storage
    let result: Option<u64> = may_load(&deps.storage, &sender_address.as_slice().to_vec()).ok().unwrap();

  

    // match result {
    //     // set all response field values
    //     Some(stored_score) => {
    //         println!("STORED SCORE IS: {}", stored_score);
    //         status = String::from("Score found.");
    //         score = Some(stored_score);

    //         Ok(ScoreResponse {
    //             score: stored_score,
    //          })
    //         // score = Some(stored_score.score);
    //         // timestamp = Some(stored_score.timestamp);
    //     }
    //     // unless there's an error
    //     None => { status = String::from("Score not found."); }
    // };

    // to_binary(&QueryAnswer::Read{ score, status })
    // Ok(StatsResponse {score_count: config.score_count, max_size: config.max_size}) // from stats_query
    Ok(ScoreResponse {
        score: result.unwrap()
    })
    // Ok(ScoreResponse {
    //    score: stored_score,
    // })
}

fn query_stats<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) ->  StdResult<StatsResponse> {
    // let state = config_read(&deps.storage).load()?;

    let config: State = load(&deps.storage, CONFIG_KEY)?;
    Ok(StatsResponse {score_count: config.score_count, max_size: config.max_size})
    // to_binary(&QueryAnswer::Stats{ score_count: config.score_count, max_size: config.max_size })
}

pub fn try_increment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
) -> StdResult<HandleResponse> {
    config(&mut deps.storage).update(|mut state| {
        state.score += 1;
        debug_print!("count = {}", state.score);
        Ok(state)
    })?;

    debug_print("count incremented successfully");
    Ok(HandleResponse::default())
}

// pub fn try_reset<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     score: i32,
// ) -> StdResult<HandleResponse> {
//     let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
//     config(&mut deps.storage).update(|mut state| {
//         if sender_address_raw != state.address {
//             return Err(StdError::Unauthorized { backtrace: None });
//         }
//         state.score = score;
//         Ok(state)
//     })?;
//     debug_print("count reset successfully");
//     Ok(HandleResponse::default())
// }

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {



    match msg {
        // QueryMsg::GetScore {} => to_binary(&query_count(deps)?),
        // QueryMsg::GetCount {} => to_binary(&query_stats(deps)?),
        QueryMsg::GetStats {} => to_binary(&query_stats(deps)?), // get the max_length allowed and the count 
        QueryMsg::GetScore { address } => to_binary(&query_read(&deps, &address)?),
        QueryMsg::WithPermit {permit, query, address}  => permit_queries(deps, permit, query, &address),
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

    let secp256k1_signature = secp256k1::Signature::from_compact(&permit.signature.signature.0)
        .map_err(|err| StdError::generic_err(format!("Malformed signature: {:?}", err)))?;
    let secp256k1_pubkey = secp256k1::PublicKey::from_slice(pubkey.0.as_slice())
        .map_err(|err| StdError::generic_err(format!("Malformed pubkey: {:?}", err)))?;

    secp256k1_verifier
        .verify(&secp256k1_msg, &secp256k1_signature, &secp256k1_pubkey)
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
    address: &HumanAddr
) -> Result<Binary, StdError> {

       // Validate permit content
       let token_address = ReadonlyConfig::from_storage(&deps.storage)
       .constants()?
       .contract_address;
    

    // let token_address = address.clone();

    let account = validate(deps, PREFIX_REVOKED_PERMITS, &permit, token_address)?;

    println!("ACCOUNT RETURNED FROM VALIDATE IS: {}", account);

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::Balance {} => {
            // if !permit.check_permission(&Permission::Balance) {
            //     return Err(StdError::generic_err(format!(
            //         "No permission to query balance, got permissions {:?}",
            //         permit.params.permissions
            //     )));
            // }

            to_binary(&query_read(&deps, &address))
        }
        // QueryWithPermit::TransferHistory { page, page_size } => {
        //     if !permit.check_permission(&Permission::History) {
        //         return Err(StdError::generic_err(format!(
        //             "No permission to query history, got permissions {:?}",
        //             permit.params.permissions
        //         )));
        //     }

        //     query_transfers(deps, &account, page.unwrap_or(0), page_size)
        // }
        // QueryWithPermit::TransactionHistory { page, page_size } => {
        //     if !permit.check_permission(&Permission::History) {
        //         return Err(StdError::generic_err(format!(
        //             "No permission to query history, got permissions {:?}",
        //             permit.params.permissions
        //         )));
        //     }

        //     query_transactions(deps, &account, page.unwrap_or(0), page_size)
        // }
        // QueryWithPermit::Allowance { address } => {
        //     if !permit.check_permission(&Permission::Allowance) {
        //         return Err(StdError::generic_err(format!(
        //             "No permission to query allowance, got permissions {:?}",
        //             permit.params.permissions
        //         )));
        //     }

        //     if account != owner && account != spender {
        //         return Err(StdError::generic_err(format!(
        //             "Cannot query allowance. Requires permit for either owner {:?} or spender {:?}, got permit for {:?}",
        //             owner.as_str(), spender.as_str(), account.as_str()
        //         )));
        //     }

        //     query_allowance(deps, owner, spender)
        // }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { max_size: 10000 };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
        
        // Query the stats 
        let res = query(&deps, QueryMsg::GetStats {}).unwrap();
        let value: StatsResponse = from_binary(&res).unwrap();
        assert_eq!(10000, value.max_size);
    }

    #[test]
    fn handle_record() {
        // FIRST WE RECORD THE SCORE
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let env = mock_env("creator", &coins(20, "token"));

        // let sender_address = deps.api.canonical_address(&address)?;
        let msg = HandleMsg::Record {score: 300};
        // let (addresses, key) = msg.get_validation_params();
        let record_res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, record_res.messages.len());


        // NEXT WE QUERY THE SCORE 
        let query_env = mock_env("creator", &coins(20, "token"));
        let query_msg = QueryMsg::GetScore { address: query_env.message.sender };
        let res = query(&deps, query_msg).unwrap(); 
        let value: ScoreResponse = from_binary(&res).unwrap();

        assert_eq!(300, value.score);
    }


    // #[test]
    // fn handle_permit_query() {
    //     // FIRST WE RECORD THE SCORE

    //     let mut deps = mock_dependencies(20, &coins(2, "token"));
    //     let name = HumanAddr("1nl7dnjcs9w2a4mn4q43nwyptf3uyllp3xh44j".to_string());
    //     let env = mock_env(name, &coins(20, "token"));

    //     // let sender_address = deps.api.canonical_address(&address)?;
    //     let msg = HandleMsg::Record {score: 300};
    //     // let (addresses, key) = msg.get_validation_params();
    //     let record_res = handle(&mut deps, env, msg).unwrap();

    //     println!("Record_res {:?}", record_res);
        
        // assert_eq!(0, record_res.messages.len());

        // let serv_addy_cannonical = deps.api.canonical_address(&env.message.sender);

        // let _env = mock_env("secret1nl7dnjcs9w2a4mn4q43nwyptf3uyllp3xh44j0", &coins(20, "token"));

        // println!("serv addy {:?}", deps.api.canonical_address(&_env.message.sender));
        // println!("Env is: {:?}", _env);

        // NEXT WE QUERY THE SCORE 

        // let permit_name = "secretswap.io";
        // let address = HumanAddr(String::from("USER"));
        // let query = QueryWithPermit::Balance {};

        

        // let permit = Permit { 
        //         params: PermitParams {
        //             allowed_tokens: vec![HumanAddr("NOT_USER".to_string())], 
        //             chain_id: "pulsar-2".to_string(), 
        //             permit_name: "secretswap.io".to_string(),
        //             permissions: vec![Permission::Balance {}]
        //           },
        //         signature: PermitSignature {
        //             pub_key: PubKey {
        //                 r#type: "tendermint/PubKeySecp256k1".to_string(),
        //                 value: to_binary("A9v535BWGzflIDgA+zepnCxuB5N3y6FJwR/jd3rIB0Ed").unwrap(),
        //              }, 
        //             signature: to_binary("2znOOKNqU1GwwrrICD8Sm7/SVQ+DcWt4Hwig+xluuTUx3EhajuNd+ds5Fqox6kWg37plpVezAo6ZtZ+iwe6KUA==").unwrap(),
        //             }
        //         }; 
        // let query_msg = QueryMsg::WithPermit { permit , query: QueryWithPermit::Balance {}, address };

                
        // let res = query(&deps, query_msg).unwrap(); 
        // let value: ScoreResponse = from_binary(&res).unwrap();

        // assert_eq!(300, value.score);
    // }

  
    // #[test]
    // fn increment() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));

    //     let msg = InitMsg { score: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();

    //     // anyone can increment
    //     let env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Increment {};
    //     let _res = handle(&mut deps, env, msg).unwrap();

    //     // should increase counter by 1
    //     let res = query(&deps, QueryMsg::GetScore {}).unwrap();
    //     let value: ScoreResponse = from_binary(&res).unwrap();
    //     assert_eq!(18, value.score);
    // }

    // #[test]
    // fn reset() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));

    //     let msg = InitMsg { score: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();

    //     // not anyone can reset
    //     let unauth_env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let res = handle(&mut deps, unauth_env, msg);
    //     match res {
    //         Err(StdError::Unauthorized { .. }) => {}
    //         _ => panic!("Must return unauthorized error"),
    //     }

    //     // only the original creator can reset the counter
    //     let auth_env = mock_env("creator", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let _res = handle(&mut deps, auth_env, msg).unwrap();

    //     // should now be 5
    //     let res = query(&deps, QueryMsg::GetScore {}).unwrap();
    //     let value: ScoreResponse = from_binary(&res).unwrap();
    //     assert_eq!(5, value.score);
    // }
}
