use candid::{CandidType, Principal};
use indexer::{BulkSnapshotIndexerHttps, Snapshot};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, Default)]
pub struct Args {
    pub id: String,
    pub ids: Vec<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
}
pub struct CalculateInput {
    pub values: Vec<f64>,
    pub value_all_assets: Vec<Vec<f64>>,
}

pub async fn call_with_transform(
    target: Principal,
    args: Args,
    transform: impl Fn(Snapshot) -> f64,
) -> Result<CalculateInput, String> {
    let indexer = BulkSnapshotIndexerHttps::new(target);
    let value = indexer.query(args.id, args.from, args.to).await?;
    let values = value
        .iter()
        .map(|x| transform(x.clone()))
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
