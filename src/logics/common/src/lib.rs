use std::collections::HashMap;

use candid::{CandidType, Principal};
use indexer::{BulkSnapshotIndexerHttps, Snapshot};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, Default)]
pub struct Args {
    pub ids: Vec<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
}
pub struct CalculateInput {
    pub value_all_assets: HashMap<String, Vec<f64>>,
}

pub async fn call_with_transform(
    target: Principal,
    args: Args,
    transform: impl Fn(Snapshot) -> f64,
) -> Result<CalculateInput, String> {
    let indexer = BulkSnapshotIndexerHttps::new(target);
    let mut value_all_assets = HashMap::new();
    for id in args.ids.clone() {
        let value = indexer
            .query(id.clone(), args.from, args.to)
            .await?
            .iter()
            .map(|x| transform(x.clone()))
            .collect();
        value_all_assets.insert(id, value);
    }
    Ok(CalculateInput { value_all_assets })
}

async fn call(target: Principal, args: Args) -> Result<CalculateInput, String> {
    call_with_transform(target, args, |x| x.value().unwrap()).await
}

pub async fn calc<T: From<CalculateInput>>(target: Principal, args: Args) -> Result<T, String>
where
    T: From<CalculateInput>,
{
    let v = call(target, args).await?;
    Ok(T::from(v))
}
