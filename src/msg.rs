use crate::viewing_key::ViewingKey;
use cosmwasm_std::{HumanAddr, StdResult};
use schemars::JsonSchema;
use secret_toolkit::permit::Permit;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub max_size: u16,
    pub prng_seed: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Record {
        score: u64,
        description: String,
    },

    WithPermit {
        permit: Permit,
        query: QueryWithPermit,
    },

    RevokePermit {
        permit_name: String,
        padding: Option<String>,
    },

    GenerateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// Return a status message to let the user know if it succeeded or failed
    Record {
        status: String,
    },

    Read {
        status: String,
        score: Option<u64>,
        description: Vec<u8>,
    },

    RevokePermit {
        status: ResponseStatus,
    },

    PermitHandle {
        data: StdResult<ScoreResponse>,
    },

    GenerateViewingKey {
        key: ViewingKey,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetScore {
        address: HumanAddr,
    },
    GetStats {},
    WithPermit {
        permit: Permit,
        query: QueryWithPermit,
    },

    Read {
        address: HumanAddr,
        key: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    Balance {},
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Read { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

// impl QueryMsg {
//     pub fn get_validation_params(&self) -> Vec<&HumanAddr> {
//         match self {
//             Self::GetScore { address, .. } => {
//                 vec![address]
//             }
//             _ => panic!("This query type does not require authentication"),
//         }
//     }
// }

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ScoreResponse {
    pub score: Option<u64>,
    pub timestamp: Option<u64>,
    pub status: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StatsResponse {
    pub score_count: u64,
    pub max_size: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub score_count: u64,
    pub max_size: u16,
    pub prng_seed: Vec<u8>,
}

/// Responses from query functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// Return basic statistics about contract
    Stats { score_count: u64, max_size: u16 },
    // Read {
    //     score: Option<u64>,
    // },
    Read {
        score: Option<u64>,
        timestamp: Option<u64>,
        description: Vec<u8>,
    },
}
