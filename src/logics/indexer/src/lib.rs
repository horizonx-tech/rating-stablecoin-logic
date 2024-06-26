use candid::{ser::IDLBuilder, CandidType, Decode, Encode, Principal};

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
        let result: Result<Option<Snapshot>, String> = raw_call_target(
            self.principal,
            "get_value",
            Encode!(&id).map_err(|e| e.to_string())?,
        )
        .await;
        result
    }
    pub async fn query(
        &self,
        id: String,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<Vec<Snapshot>, String> {
        let opts = QueryOptions {
            from_timestamp: from,
            to_timestamp: to,
        };
        let args: Vec<u8> = IDLBuilder::new()
            .arg(&id)
            .map_err(|e| e.to_string())?
            .arg(&opts)
            .map_err(|e| e.to_string())?
            .serialize_to_vec()
            .map_err(|e| e.to_string())?;
        raw_call_target(self.principal, "query_between", args).await
    }
}
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
    id: SnapshotId,
    value: SnapshotValue,
    timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct SnapshotValue {
    raw: Vec<u8>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct QueryOptions {
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>,
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
    pub fn value_from_string(&self) -> Option<f64> {
        let value_string: Result<DexValue, bincode::Error> =
            bincode::deserialize(&self.value.raw.as_slice());
        if value_string.is_err() {
            ic_cdk::println!("Failed to deserialize value: {:?}", value_string.err());
            return None;
        }
        Some(value_string.unwrap().v.parse().unwrap())
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
#[derive(Deserialize, Serialize)]
struct DexValue {
    v: String,
}
async fn raw_call_target<T: CandidType + DeserializeOwned>(
    target: Principal,
    method_name: &str,
    args: Vec<u8>,
) -> Result<T, String> {
    let result: Result<Vec<u8>, (ic_cdk::api::call::RejectionCode, String)> =
        ic_cdk::api::call::call_raw(target, method_name, args, 0).await;
    match result {
        Ok(bytes) => {
            Decode!(bytes.as_slice(), T).map_err(|e| format!("Error decoding response: {:?}", e))
        }
        Err((code, msg)) => Err(format!("Error: {:?}, {:?}", code, msg)),
    }
}
