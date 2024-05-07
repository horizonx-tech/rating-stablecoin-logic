use std::collections::HashMap;

use bindings::Args;
use bindings::LensArgs;
use candid::CandidType;
use candid::Principal;
use chainsight_cdk_macros::StableMemoryStorable;
use ic_web3_rs::ic;
use rating_indexer_bindings as bindings;
use serde::{Deserialize, Serialize};
use ulid_lib::Ulid;
pub type ResponseType = bindings::ResponseType;
pub type RequestArgsType = bindings::RequestArgsType;
use crate::calculator::ScoreCalculator;
use crate::Decode;
use crate::Encode;
const NANOS_PER_MILLIS: u64 = 1_000_000;
const NANOS_PER_SEC: u64 = 1_000_000_000;
#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize, Debug, PartialEq)]
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = true)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub value: SnapshotValue,
    pub scores: HashMap<TaskId, SnapshotValue>,
}

impl Snapshot {
    pub async fn from(scores: HashMap<TaskId, (SnapshotValue, CalculationStragety)>) -> Self {
        let id = SnapshotId::new().await;
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
    pub args: TaskArgs,
    pub source: Principal,
    pub strategy: CalculationStragety,
}

impl Task {
    pub fn to_lens_args(&self, duration_secs: u64) -> LensArgs {
        let now = ic_cdk::api::time();
        let duration_nanosecs = duration_secs * NANOS_PER_SEC;
        let from = (now - duration_nanosecs) as i64;
        LensArgs {
            args: Args {
                from: Some(from),
                to: Some(now as i64),
                id: self.args.id.clone(),
                ids: self.args.ids.clone(),
            },
            targets: vec![self.source.clone().to_string()],
        }
    }
}
#[derive(CandidType, Clone, StableMemoryStorable, Serialize, Deserialize)]
pub struct TaskArgs {
    pub id: String,
    pub ids: Vec<String>,
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
#[stable_mem_storable_opts(max_size = 10000, is_fixed_size = false)]
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
    pub async fn new() -> Self {
        let now_msec = ic_cdk::api::time() / NANOS_PER_MILLIS;
        let rand = raw_rand().await.unwrap();
        let id = Ulid::from_parts(now_msec, u128::from(rand[0]));
        Self { id: id.to_string() }
    }
    #[cfg(test)]
    pub async fn new() -> Self {
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
#[cfg(not(test))]
pub async fn raw_rand() -> Result<Vec<u8>, String> {
    let (rand_msb,): (Vec<u8>,) =
        ic_cdk::api::call::call(Principal::management_canister(), "raw_rand", ())
            .await
            .map_err(|e| format!("{:?}", e))?;
    Ok(rand_msb)
}

#[cfg(test)]
mod tests {
    use ulid_lib::Ulid;

    #[test]
    fn test_ulid_from() {
        let now_nanosec = 1715055275000_i64 * 1_000_000;
        let rand = now_nanosec as u128;
        let id = Ulid::from_parts((now_nanosec / 1_000_000) as u64, rand);
        assert_eq!(id.timestamp_ms(), 1715055275000);
    }
}
