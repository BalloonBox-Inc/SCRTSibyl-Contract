use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, HumanAddr
};

use std::convert::TryFrom;

use crate::msg::{ScoreResponse, HandleMsg, InitMsg, QueryMsg, HandleAnswer, QueryAnswer, StatsResponse};
use crate::state::{config, config_read, User, save, may_load, State, CONFIG_KEY, load};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
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


// fn query_read<S: Storage, A: Api, Q: Querier>(
//     deps: &Extern<S, A, Q>,
//     address: &HumanAddr,
// ) -> StdResult<Binary> {
//     let status: String;
//     let mut score: Option<String> = None;
//     // let mut timestamp: Option<u64> = None;

//     let sender_address = deps.api.canonical_address(&address)?;

//     // read the reminder from storage
//     let result: Option<Score> = may_load(&deps.storage, &sender_address.as_slice().to_vec()).ok().unwrap();
//     match result {
//         // set all response field values
//         Some(stored_score) => {
//             status = String::from("Score found.");
//             // score = String::from_utf8(stored_score.score).ok();
//             // score = Some(stored_score.score);
//             // timestamp = Some(stored_score.timestamp);
//         }
//         // unless there's an error
//         None => { status = String::from("Score not found."); }
//     };

//     to_binary(&QueryAnswer::Read{ status, score, timestamp })
// }

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
) -> StdResult<Binary> {
    match msg {
        // QueryMsg::GetScore {} => to_binary(&query_count(deps)?),
        // QueryMsg::GetCount {} => to_binary(&query_stats(deps)?),
        QueryMsg::GetStats {} => to_binary(&query_stats(deps)?), // get the max_length allowed and the count 
        QueryMsg::GetScore {} => to_binary("The Score has been queried"),
    }
}

fn query_stats<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) ->  StdResult<StatsResponse> {
    // let state = config_read(&deps.storage).load()?;

    let config: State = load(&deps.storage, CONFIG_KEY)?;

    println!("CONFIG IS: {:?}", config);
    Ok(StatsResponse {score_count: config.score_count, max_size: config.max_size})
    // to_binary(&QueryAnswer::Stats{ score_count: config.score_count, max_size: config.max_size })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { max_size: 1000 };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();

        println!("Res is {:?}", res);

        assert_eq!(0, res.messages.len());
        
        // Query the stats 
        let res = query(&deps, QueryMsg::GetStats {}).unwrap();
        println!("res is: {:?}", res);
        
        let value: StatsResponse = from_binary(&res).unwrap();
        println!("val is: {:?}", value);

        println!("Max size is {:?}", value.max_size);
        assert_eq!(1000, value.max_size);
    }

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
    // fn handle_try_record() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));

    //     // anyone can increment
    //     let env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Record {score: 300};
    //     let _res = handle(&mut deps, env, msg).unwrap();

    //     let data = _res.data;
        
    //     println!("_res data  is : {:?}", data);
        
    //     // this is breaking! 
    //     let res = query(&deps, QueryMsg::GetScore {}).unwrap(); 
        
    //     let value: ScoreResponse = from_binary(&res).unwrap();
  
    //     println!("try record test score is {}", value.score);
    //     assert_eq!(300, value.score);
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
