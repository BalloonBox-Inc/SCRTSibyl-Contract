use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr};
use secret_toolkit::permit::Permit;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub max_size: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // Increment {},
    // Reset { count: i32 },
    Record {
        score: u64
    },
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
    }
  
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
        address: HumanAddr // adding the address of the user 
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    Balance {
        // address: HumanAddr
    },
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> Vec<&HumanAddr> {



        match self {
            Self::GetScore { address, .. } => {
                println!("address: {}", address);
                println!("validation params : {:?}", vec![address]);
                vec![address]
            },
            _ => panic!("This query type does not require authentication"),
        }
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ScoreResponse {
    pub score: u64,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StatsResponse {
    pub score_count: u64,
    pub max_size: u16 
}


/// Responses from query functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// Return basic statistics about contract
    Stats {
        score_count: u64,
        max_size: u16,
    },
    /// Return a status message and the current reminder and its timestamp, if it exists
    Read {
        score: Option<u64>,
        // status: String,
        // reminder: Option<String>,
        // timestamp: Option<u64>,
    },
}
