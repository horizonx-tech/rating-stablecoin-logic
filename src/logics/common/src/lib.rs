use candid::{CandidType, Principal};
use indexer::BulkSnapshotIndexerHttps;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, Default)]
pub struct Args {
    pub id: String,
    pub ids: Vec<String>,
    pub from: i64,
    pub to: i64,
}
pub struct CalculateInput {
    pub values: Vec<f64>,
    pub value_all_assets: Vec<Vec<f64>>,
}

async fn calculate(target: Principal, args: Args) -> Result<CalculateInput, String> {
    let indexer = BulkSnapshotIndexerHttps::new(target);
    let value = indexer.query(args.id, args.from, args.to).await?;
    let values = value
        .iter()
        .map(|x| x.value().unwrap())
        .collect::<Vec<f64>>();
    let mut value_all_assets = vec![];
    for id in args.ids {
        let value = indexer.query(id, args.from, args.to).await?;
        let values = value
            .iter()
            .map(|x| x.value().unwrap())
            .collect::<Vec<f64>>();
        value_all_assets.push(values);
    }
    Ok(CalculateInput {
        values,
        value_all_assets,
    })
}

pub async fn calc<T: From<CalculateInput>>(target: Principal, args: Args) -> Result<T, String>
where
    T: From<CalculateInput>,
{
    let v = calculate(target, args).await?;
    Ok(T::from(v))
}
