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

#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub value: HashMap<String, f64>,
}

pub struct CalculateInput {
    pub value_all_assets: HashMap<String, Vec<f64>>,
}

pub async fn call_and_transform(
    target: Principal,
    args: Args,
    transform_func: impl Fn(Snapshot) -> f64,
) -> Result<CalculateInput, String> {
    let indexer = BulkSnapshotIndexerHttps::new(target);
    let mut value_all_assets = HashMap::new();
    for id in args.ids.clone() {
        let value = indexer
            .query(id.clone(), args.from, args.to)
            .await?
            .iter()
            .map(|x| transform_func(x.clone()))
            .collect();
        value_all_assets.insert(id, value);
    }
    Ok(CalculateInput { value_all_assets })
}

async fn call(target: Principal, args: Args) -> Result<CalculateInput, String> {
    call_and_transform(target, args, |x| x.value().unwrap()).await
}

pub async fn call_and_score(
    target: Principal,
    args: Args,
    score_func: impl Fn(&[f64], &[Vec<f64>]) -> f64,
) -> Result<LensValue, String> {
    let v = call(target, args).await?;
    Ok(score_from_input(v, score_func))
}

pub fn score_from_input(
    v: CalculateInput,
    score_func: impl Fn(&[f64], &[Vec<f64>]) -> f64,
) -> LensValue {
    let value_all_assets = v.value_all_assets;
    let values_all_assets: Vec<Vec<f64>> = value_all_assets.values().cloned().collect();
    let value = value_all_assets
        .iter()
        .map(|(k, v)| (k.clone(), score_func(v, &values_all_assets)))
        .collect();
    LensValue { value }
}
