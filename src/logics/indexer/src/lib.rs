use candid::{CandidType, Principal};
use ic_cdk::api::call::CallResult;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct BulkSnapshotIndexerHttps {
    pub principal: Principal,
}
impl BulkSnapshotIndexerHttps {
    pub fn new(principal: Principal) -> Self {
        Self { principal }
    }
    pub async fn get_value(&self, id: String) -> Result<Option<Snapshot>, String> {
        raw_call_target(self.principal, "get_value", id).await?
    }
    pub async fn query(&self, id: String, from: i64, to: i64) -> Result<Vec<Snapshot>, String> {
        let opts = QueryOptions {
            from_timestamp: Some(from),
            to_timestamp: Some(to),
        };
        raw_call_target(self.principal, "query_between", (id, opts)).await?
    }
}
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct QueryOptions {
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>,
}

#[derive(CandidType, Serialize, Clone, Debug, Deserialize)]
pub struct SnapshotValue {
    raw: Vec<u8>,
}

#[derive(CandidType, Serialize, Clone, Debug, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    value: SnapshotValue,
    pub timestamp: u64,
}

impl Snapshot {
    pub fn value(&self) -> Option<f64> {
        let value_f64: Result<Value, bincode::Error> =
            bincode::deserialize(&self.value.raw.as_slice());
        if value_f64.is_err() {
            ic_cdk::println!("Failed to deserialize value: {:?}", value_f64.err());
            return None;
        }
        Some(value_f64.unwrap().v)
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SnapshotId {
    pub id: String,
}

#[derive(Deserialize, Serialize)]
struct Value {
    v: f64,
}

async fn raw_call_target<T: CandidType + DeserializeOwned, A: CandidType>(
    target: Principal,
    method_name: &str,
    args: A,
) -> Result<T, String> {
    let result: CallResult<(T,)> = ic_cdk::api::call::call(target, method_name, (args,)).await;

    if result.is_err() {
        ic_cdk::println!("Failed to call {}: {:?}", target, result.err());
        return Err("Failed to call".to_string());
    }
    Ok(result.unwrap().0)
}
