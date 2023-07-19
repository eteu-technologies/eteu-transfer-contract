use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema))]
#[serde(crate = "near_sdk::serde")]
pub struct TokenCreateMetadata {
    pub title: String,
    pub description: String,
}
