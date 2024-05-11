use std::collections::HashMap;

mod types;
use candid::{CandidType, Principal};
use indexer::{BulkSnapshotIndexerHttps, Snapshot};
use serde::{Deserialize, Serialize};
use types::Date;

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
    let mut snapshots: HashMap<String, Vec<Snapshot>> = HashMap::new();
    let mut value_all_assets: HashMap<String, Vec<f64>> = HashMap::new();
    let mut ts_first: u64 = u64::MAX;
    let mut ts_last: u64 = 0;
    for id in args.ids.clone() {
        let value: Vec<Snapshot> = indexer
            .query(id.clone(), args.from, args.to)
            .await?
            .iter()
            .map(|x| {
                ts_first = ts_first.min(x.timestamp);
                ts_last = ts_last.max(x.timestamp);
                return x.clone();
            })
            .collect();
        snapshots.insert(id.clone(), value);
    }
    snapshots.iter().for_each(|(k, v)| {
        let complemented = complement_datasets(v.clone(), ts_first, ts_last);
        let values = complemented
            .iter()
            .map(|x| transform_func(x.clone()))
            .collect();
        value_all_assets.insert(k.clone(), values);
    });
    Ok(CalculateInput { value_all_assets })
}

fn complement_datasets(datasets: Vec<Snapshot>, ts_first: u64, ts_last: u64) -> Vec<Snapshot> {
    let date_from = Date::from(ts_first);
    let date_to = Date::from(ts_last);
    let mut result: HashMap<Date, Snapshot> = HashMap::new();
    datasets.iter().for_each(|x| {
        let date = Date::from(x.timestamp);
        result.insert(date, x.clone());
    });
    let mut start_date = date_from;
    while start_date < date_to {
        result.entry(start_date).or_insert(Snapshot::default());
        start_date = start_date.clone().next();
    }
    result.entry(date_to).or_insert(Snapshot::default());
    result
        .iter()
        .map(|(_, v)| v.clone())
        .collect::<Vec<Snapshot>>()
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

#[cfg(test)]
mod test {
    use indexer::{SnapshotId, SnapshotValue};
    const NANOSECONDS_IN_DAY: u64 = 86_400_000_000_000;
    #[test]
    fn test_complement_datasets() {
        let v = SnapshotValue { raw: vec![] };
        use super::*;
        let ts_first = 0 * NANOSECONDS_IN_DAY;
        let ts_last = 4 * NANOSECONDS_IN_DAY;
        let datasets = vec![
            Snapshot {
                id: SnapshotId {
                    id: "1".to_string(),
                },
                value: v.clone(),
                timestamp: 1 * NANOSECONDS_IN_DAY,
            },
            Snapshot {
                id: SnapshotId {
                    id: "2".to_string(),
                },
                value: v.clone(),
                timestamp: 3 * NANOSECONDS_IN_DAY,
            },
        ];
        let result = complement_datasets(datasets, ts_first, ts_last);
        assert_eq!(result.len(), 5);
    }
}
