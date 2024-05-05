use std::collections::HashMap;

use candid::CandidType;
use candid::Principal;
use chainsight_cdk_macros::StableMemoryStorable;
use rating_indexer_bindings as bindings;
use serde::{Deserialize, Serialize};
use ulid_lib::Ulid;
pub type ResponseType = bindings::ResponseType;
pub type RequestArgsType = bindings::RequestArgsType;
use crate::calculator::ScoreCalculator;
use crate::Decode;
use crate::Encode;
#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize, Debug, PartialEq)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub value: SnapshotValue,
    pub scores: HashMap<TaskId, SnapshotValue>,
}

impl From<HashMap<TaskId, (SnapshotValue, CalculationStragety)>> for Snapshot {
    fn from(scores: HashMap<TaskId, (SnapshotValue, CalculationStragety)>) -> Self {
        let id = SnapshotId::new();
        let values = scores
            .values()
            .into_iter()
            .map(|(v, s)| (*v, Some(s.weight)))
            .collect();
        let value = ScoreCalculator::new().calculate(values);
        let scores = scores.into_iter().map(|(k, (v, _))| (k, v)).collect();
        Self { id, value, scores }
    }
}
pub type SnapshotValue = f64;
type TaskId = String;

#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct Task {
    pub id: TaskId,
    pub lens: Principal,
    pub args: RequestArgsType,
    pub source: Principal,
    pub strategy: CalculationStragety,
}
#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize)]
pub struct CalculationStragety {
    pub weight: f64,
}
impl Default for CalculationStragety {
    fn default() -> Self {
        Self { weight: 1.0 }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize)]
pub struct QueryOption {
    pub from_timestamp: Option<i64>,
    pub to_timestamp: Option<i64>,
}
#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize, Debug)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct SnapshotId {
    pub id: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, StableMemoryStorable)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = false)] // temp: max_size
pub struct SnapshotIds {
    ids: Vec<SnapshotId>,
}
impl SnapshotId {
    #[cfg(not(test))]
    pub fn new() -> Self {
        let now_msec = ic_cdk::api::time() / 1_000_000;
        let random_u128 = now_msec as u128;
        let id = Ulid::from_parts(now_msec, random_u128);
        Self { id: id.to_string() }
    }
    #[cfg(test)]
    pub fn new() -> Self {
        let time = 1234567890;
        Self {
            id: Ulid::from_parts(time, 0).to_string(),
        }
    }
}

impl Ord for SnapshotId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_id = Ulid::from_string(&self.id).expect("Invalid id");
        let other_id = Ulid::from_string(&other.id).expect("Invalid id");
        self_id.cmp(&other_id)
    }
}
impl PartialOrd for SnapshotId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_id = Ulid::from_string(&self.id).expect("Invalid id");
        let other_id = Ulid::from_string(&other.id).expect("Invalid id");
        self_id.partial_cmp(&other_id)
    }
}
impl PartialEq for SnapshotId {
    fn eq(&self, other: &Self) -> bool {
        let self_id = Ulid::from_string(&self.id).expect("Invalid id");
        let other_id = Ulid::from_string(&other.id).expect("Invalid id");
        self_id.eq(&other_id)
    }
}
impl Eq for SnapshotId {
    fn assert_receiver_is_total_eq(&self) {
        let self_id = Ulid::from_string(&self.id).expect("Invalid id");
        self_id.assert_receiver_is_total_eq()
    }
}

#[derive(
    CandidType,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    StableMemoryStorable,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct Key {
    pub id: String,
}

impl From<String> for Key {
    fn from(id: String) -> Self {
        Key { id }
    }
}
impl Into<String> for Key {
    fn into(self) -> String {
        self.id
    }
}
#[derive(
    CandidType,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    StableMemoryStorable,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct Value {
    pub value: String,
}

macro_rules! bitmask {
    ($len:expr) => {
        ((1 << $len) - 1)
    };
}

async fn raw_rand() -> Result<Vec<u8>, String> {
    let (rand_msb,): (Vec<u8>,) =
        ic_cdk::api::call::call(Principal::management_canister(), "raw_rand", ())
            .await
            .map_err(|e| format!("{:?}", e))?;
    Ok(rand_msb)
}

pub async fn ulid_from_unix_epoch(epoch: i64) -> Ulid {
    let timestamp = epoch;
    let timebits = (timestamp & bitmask!(Ulid::TIME_BITS)) as u64;
    let rand_msb = raw_rand().await.unwrap();
    let msb = timebits << 16 | u64::from(rand_msb[0]);
    let rand_lsb = raw_rand().await.unwrap();
    let lsb = u64::from(rand_lsb[0]);
    Ulid::from((msb, lsb))
}

pub async fn ulid_from_datetime(from: i64) -> Ulid {
    ulid_from_unix_epoch(from).await
}
