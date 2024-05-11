use std::cell::RefCell;
use std::collections::HashMap;

use candid::CandidType;
use candid::{Decode, Encode, Principal};
use chainsight_cdk::rpc::AsyncReceiverProvider;
use chainsight_cdk::rpc::Caller;
use chainsight_cdk::rpc::Receiver;
use chainsight_cdk::rpc::ReceiverProvider;
use chainsight_cdk::rpc::ReceiverProviderWithoutArgs;
use chainsight_cdk::rpc::{CallProvider, Message};
use chainsight_cdk_macros::{
    chainsight_common, did_export, init_in, prepare_stable_structure, stable_memory_for_scalar,
    timer_task_func, StableMemoryStorable,
};

use ic_stable_structures::{memory_manager::MemoryId, BTreeMap};
use ic_web3_rs::futures::future::join_all;
use ic_web3_rs::futures::future::BoxFuture;
use ic_web3_rs::futures::FutureExt;
use rating_indexer::CallCanisterArgs;
use rating_indexer_bindings::{Args, LensArgs, LensValue, Sources};
use serde::{Deserialize, Serialize};
use types::{AggregationKey, IdToFetch, TaskId};
use types::{Key, QueryOption, Snapshot, SnapshotId, SnapshotValue, Task, Value};
use ulid_lib::Ulid;
mod calculator;
use crate::calculator::ScoreCalculator;

mod types;
chainsight_common!();
init_in!(2);
timer_task_func!("set_task", "index", 6);
prepare_stable_structure!();

macro_rules! only_proxy {
    () => {
        if ic_cdk::api::caller() != get_proxy() {
            ic_cdk::trap("Unauthorized");
        }
    };
}

macro_rules! only_controller {
    () => {
        if !ic_cdk::api::is_controller(&ic_cdk::api::caller()) {
            ic_cdk::trap("Unauthorized");
        }
    };
}
thread_local! {
    static SNAPSHOTS: RefCell<BTreeMap<SnapshotId, Snapshot, MemoryType>> =  RefCell::new(
        BTreeMap::init(
            MEMORY_MANAGER.with(|mm|mm.borrow().get(MemoryId::new(6)))
        )
    );

    static TASKS: RefCell<BTreeMap<Key, Task, MemoryType>> = RefCell::new(
        BTreeMap::init(
            MEMORY_MANAGER.with(|mm|mm.borrow().get(MemoryId::new(7)))
        )
    );
    static SNAPSHOT_IDS: RefCell<ic_stable_structures::Vec<SnapshotId,MemoryType>> = RefCell::new(
        ic_stable_structures::Vec::init(
            MEMORY_MANAGER.with(|mm|mm.borrow().get(MemoryId::new(8)))).unwrap()
    );
    static CONFIGS: RefCell<BTreeMap<Key,Value,MemoryType>> = RefCell::new(
        BTreeMap::init(
            MEMORY_MANAGER.with(|mm|mm.borrow().get(MemoryId::new(9)))
        )
    );
}

#[ic_cdk::update]
#[candid::candid_method(update)]
fn setup() -> Result<(), String> {
    Ok(())
}

#[ic_cdk::update]
#[candid::candid_method(update)]
fn add_task(task: Task) {
    only_controller!();
    TASKS.with(|tasks| {
        tasks.borrow_mut().insert(
            Key {
                id: task.id.clone(),
            },
            task,
        );
    });
}

#[ic_cdk::update]
#[candid::candid_method(update)]
fn remove_task(id: String) {
    only_controller!();
    TASKS.with(|tasks| {
        tasks.borrow_mut().remove(&Key { id });
    });
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn tasks() -> Vec<Task> {
    let mut results = vec![];
    TASKS.with(|tasks| {
        for task in tasks.borrow().iter() {
            results.push(task.1.clone());
        }
    });
    results
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn max_count() -> u64 {
    CONFIGS.with(|configs| {
        let max_count = configs.borrow().get(&Key::from("max_count".to_string()));
        if max_count.is_none() {
            return 1000;
        }
        let max_count = max_count.unwrap();
        let max_count = max_count.value.parse::<u64>();
        if max_count.is_err() {
            return 1000;
        }
        max_count.unwrap()
    })
}

fn _duration_seconds() -> u64 {
    CONFIGS.with(|configs| {
        let duration_seconds = configs
            .borrow()
            .get(&Key::from("duration_seconds".to_string()));
        if duration_seconds.is_none() {
            return 60 * 60 * 24; // 1 day
        }
        let duration_seconds = duration_seconds.unwrap();
        let duration_seconds = duration_seconds.value.parse::<u64>();
        if duration_seconds.is_err() {
            return 60 * 60 * 24; // 1 day
        }
        duration_seconds.unwrap()
    })
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn duration_seconds() -> u64 {
    _duration_seconds()
}

#[ic_cdk::update]
#[candid::candid_method(update)]
fn update_duration_seconds(duration_seconds: u64) {
    only_controller!();
    _update_duration_seconds(duration_seconds);
}

fn _update_duration_seconds(duration_seconds: u64) {
    CONFIGS.with(|configs| {
        configs.borrow_mut().insert(
            Key::from("duration_seconds".to_string()),
            Value {
                value: duration_seconds.to_string(),
            },
        );
    });
}

#[ic_cdk::update]
#[candid::candid_method(update)]
fn update_max_count(count: u64) {
    only_controller!();
    _update_max_count(count);
}

fn _update_max_count(count: u64) {
    CONFIGS.with(|configs| {
        configs.borrow_mut().insert(
            Key::from("max_count".to_string()),
            Value {
                value: count.to_string(),
            },
        );
    });
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn call_args() -> LensArgs {
    LensArgs {
        args: Args {
            from: None,
            to: None,
            ids: vec![],
        },
        targets: vec![],
    }
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_last_snapshot() -> Snapshot {
    SNAPSHOTS.with(|snapshots| snapshots.borrow().last_key_value().unwrap().1.clone())
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_last_snapshot_value() -> SnapshotValue {
    get_last_snapshot().value
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_snapshots() -> Vec<Snapshot> {
    // unsupported
    vec![]
}

#[ic_cdk::query]
#[candid::candid_method(query)]
async fn query_between(opt: QueryOption) -> Vec<Snapshot> {
    _query_between(opt).await
}

fn _query_between(opt: QueryOption) -> BoxFuture<'static, Vec<Snapshot>> {
    let from = opt.from_timestamp.unwrap_or(0);
    let to = opt.to_timestamp.unwrap_or(0);
    let divisor = 1_000_000; // nanosec to msec
    async move { snapshots_between((from / divisor) as u64, (to / divisor) as u64).await }.boxed()
}

async fn snapshots_between(from: u64, to: u64) -> Vec<Snapshot> {
    let mut result = vec![];
    let mut ids_to_fetch = vec![];
    SNAPSHOT_IDS.with(|id| {
        let snapshot_len = id.borrow().len();
        for idx in 0..snapshot_len {
            let id = id.borrow().get(snapshot_len - idx - 1).unwrap();
            let ulid = Ulid::from_string(&id.id).unwrap();
            if ulid.timestamp_ms().lt(&from) {
                break;
            }
            if ulid.timestamp_ms().le(&to) {
                ids_to_fetch.push(ulid);
            }
        }
        for id in ids_to_fetch {
            let snapshot = SNAPSHOTS
                .with(|snapshots| snapshots.borrow().get(&SnapshotId { id: id.to_string() }));
            if let Some(snapshot) = snapshot {
                result.push(snapshot.clone());
            }
        }
    });
    result
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_sources() -> Vec<Sources> {
    vec![]
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_snapshot(_: u64) -> Snapshot {
    // unsupported
    get_last_snapshot()
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_snapshot_value(_: u64) -> SnapshotValue {
    // unsupported
    get_last_snapshot_value()
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn snapshots_len() -> u64 {
    SNAPSHOT_IDS.with(|id| id.borrow().len() as u64)
}

#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_top_snapshots(top: u64) -> Vec<Snapshot> {
    let mut result = vec![];
    let mut ids_to_fetch = vec![];
    SNAPSHOT_IDS.with(|id| {
        for idx in id.borrow().len()..0 {
            let id = id.borrow().get(idx).unwrap();
            ids_to_fetch.push(id);
            if ids_to_fetch.len() >= top as usize {
                break;
            }
        }
    });
    for id in ids_to_fetch {
        let snapshot =
            SNAPSHOTS.with(|snapshots| snapshots.borrow().get(&SnapshotId { id: id.id }));
        if let Some(snapshot) = snapshot {
            result.push(snapshot);
        }
    }
    result
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_top_snapshot_values(top: u64) -> Vec<SnapshotValue> {
    get_top_snapshots(top)
        .into_iter()
        .map(|s| s.value)
        .collect()
}

fn _delete_snapshots() {
    let max_count = max_count();
    let len = snapshots_len();
    if len <= max_count {
        return;
    }
    _delete((len - max_count) as usize);
}

fn _delete(size: usize) {
    if size == 0 {
        return;
    }
    SNAPSHOT_IDS.with(|ids| {
        let ids = ids.borrow_mut();
        let new_ids = ids
            .iter()
            .skip(size)
            .map(|id| id.clone())
            .collect::<Vec<SnapshotId>>();
        let mut deletion_ids = vec![];
        for id in ids.iter().take(size) {
            deletion_ids.push(id.clone());
        }
        let mut idx = 0;
        for id in new_ids {
            ids.set(idx, &id);
            idx += 1;
        }
        let deletion_count = ids.len() - idx;
        for _ in 0..deletion_count {
            ids.pop();
        }
        SNAPSHOTS.with(|snapshots| {
            for id in deletion_ids {
                snapshots.borrow_mut().remove(&id);
            }
        });
    });
}

fn _add_snapshot(snapshot: Snapshot) {
    SNAPSHOTS.with(|snapshots| {
        snapshots
            .borrow_mut()
            .insert(snapshot.id.clone(), snapshot.clone());
    });
    SNAPSHOT_IDS.with(|ids| {
        ids.borrow_mut().push(&snapshot.id.clone()).unwrap();
    });
    _delete_snapshots();
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn test_index() {
    _index().await.unwrap();
}

fn weight(key: Key, fetch_key: String) -> f64 {
    TASKS.with(|tasks| {
        let default = 1.0;
        let task = tasks.borrow().get(&key);
        if task.is_none() {
            return default;
        }
        let task = task.unwrap();
        let options = task.options.options;
        for opt in options {
            if opt.id_to_fetch.eq(&fetch_key) {
                return opt.strategy.weight;
            }
        }
        default
    })
}
fn aggregation_key(key: Key, id_to_fetch: IdToFetch) -> AggregationKey {
    TASKS.with(|tasks| {
        let default = "".to_string();
        let task = tasks.borrow().get(&key);
        if task.is_none() {
            return default;
        }
        let task = task.unwrap();
        let options = task.options.options;
        for opt in options {
            if opt.id_to_fetch.eq(&id_to_fetch) {
                return opt.aggregation_key;
            }
        }
        default
    })
}

async fn _index() -> Result<(), String> {
    let mut futures = vec![];
    let mut task_ids = vec![];
    let duration = duration_seconds();
    TASKS.with(|tasks| {
        for (key, task) in tasks.borrow().iter() {
            task_ids.push(key.clone());
            let future = async move {
                let out = call(task.lens, task.to_lens_args(duration)).await.unwrap();
                let mut scores = HashMap::new();
                for (k, v) in out.clone().iter() {
                    let weight = weight(key.clone(), k.clone());
                    scores.insert(k.to_string(), (v.to_owned(), weight));
                }
                scores
            };
            futures.push(future);
        }
    });
    let results = join_all(futures).await;
    let mut results_by_assets: HashMap<AggregationKey, HashMap<TaskId, (f64, f64)>> =
        HashMap::new();
    for (idx, result) in results.iter().enumerate() {
        let key = task_ids.get(idx).unwrap();
        let task = TASKS.with(|tasks| tasks.borrow().get(key).unwrap());
        result.iter().for_each(|(k, v)| {
            let id_to_fetch = k.clone();
            let (value, weight) = v.clone();
            let task_id = task.id.clone();
            let aggregation_key = aggregation_key(key.clone(), id_to_fetch.clone());
            let scores = results_by_assets.get(&aggregation_key);
            if scores.is_none() {
                let mut new_scores = HashMap::new();
                new_scores.insert(task_id.clone(), (value.unwrap_or_default(), weight));
                results_by_assets.insert(aggregation_key.clone(), new_scores);
            }
            let scores = results_by_assets.get_mut(&aggregation_key).unwrap();
            scores.insert(task_id.clone(), (value.unwrap_or_default(), weight));
        });
    }
    let mut score = HashMap::new();
    let mut scores: HashMap<AggregationKey, HashMap<TaskId, f64>> = HashMap::new();
    results_by_assets.clone().iter().for_each(|r| {
        let s = r.1.clone();
        let mut new_scores: HashMap<TaskId, f64> = HashMap::new();
        let values = s
            .iter()
            .map(|(k, v)| {
                new_scores.insert(k.clone(), v.0);
                return (v.0, Some(v.1));
            })
            .collect();
        score.insert(r.0.clone(), ScoreCalculator::new().calculate(values));
        scores.insert(r.0.clone(), new_scores);
    });
    let snapshot = Snapshot {
        id: SnapshotId::new().await,
        value: score,
        scores,
    };
    _add_snapshot(snapshot);

    Ok(())
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn index() {
    only_proxy!();
    _index().await.unwrap();
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
struct LensValueExt {
    value: HashMap<String, Option<f64>>,
}

async fn call(lens: Principal, args: LensArgs) -> Result<HashMap<String, Option<f64>>, String> {
    let method_name = "proxy_get_result";
    let px = _get_target_proxy(lens).await;
    let result = CallProvider::new()
        .call(
            Message::new::<CallCanisterArgs>(args, px.clone(), &method_name)
                .map_err(|e| format!("failed to encode message: {:?}", e))?,
        )
        .await
        .map_err(|e| format!("failed to call: {:?}", e))?;
    result
        .reply::<LensValueExt>()
        .map_err(|e| format!("failed to decode reply: {:?}", e))
        .map(|v| v.value)
}

/// proxy methods
#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_last_snapshot(input: Vec<u8>) -> Vec<u8> {
    ReceiverProviderWithoutArgs::<Snapshot>::new(proxy(), get_last_snapshot)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_last_snapshot_value(input: Vec<u8>) -> Vec<u8> {
    ReceiverProviderWithoutArgs::<SnapshotValue>::new(proxy(), get_last_snapshot_value)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_snapshot(input: Vec<u8>) -> Vec<u8> {
    ReceiverProvider::<u64, Snapshot>::new(proxy(), get_snapshot)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_snapshot_value(input: Vec<u8>) -> Vec<u8> {
    ReceiverProvider::<u64, SnapshotValue>::new(proxy(), get_snapshot_value)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_snapshots(input: Vec<u8>) -> Vec<u8> {
    ReceiverProviderWithoutArgs::<Vec<Snapshot>>::new(proxy(), get_snapshots)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_top_snapshot_values(input: Vec<u8>) -> Vec<u8> {
    ReceiverProvider::<u64, Vec<SnapshotValue>>::new(proxy(), get_top_snapshot_values)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_get_top_snapshots(input: Vec<u8>) -> Vec<u8> {
    ReceiverProvider::<u64, Vec<Snapshot>>::new(proxy(), get_top_snapshots)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_snapshots_len(input: Vec<u8>) -> Vec<u8> {
    ReceiverProviderWithoutArgs::<u64>::new(proxy(), snapshots_len)
        .reply(input)
        .await
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn proxy_query_between(input: Vec<u8>) -> Vec<u8> {
    AsyncReceiverProvider::<QueryOption, Vec<Snapshot>>::new(proxy(), _query_between)
        .reply(input)
        .await
}

#[cfg(test)]
mod test2 {

    use std::collections::HashMap;

    use super::*;
    #[test]
    fn test_add_snapshot() {
        let snapshot = Snapshot {
            id: SnapshotId {
                id: "01HX8MP06M000000000000004F".to_string(),
            },
            value: HashMap::new(),
            scores: HashMap::new(),
        };
        _add_snapshot(snapshot.clone());
        let result = SNAPSHOTS.with(|snapshots| snapshots.borrow().get(&snapshot.id));
        assert_eq!(result.unwrap().id, snapshot.id);
        let ids_result = SNAPSHOT_IDS.with(|ids| ids.borrow().len());
        assert_eq!(ids_result, 1);
    }

    #[test]
    fn test_delete_snapshot() {
        let snapshot = Snapshot {
            id: SnapshotId {
                id: "01HX8MP06M000000000000004F".to_string(),
            },
            value: HashMap::new(),
            scores: HashMap::new(),
        };
        _update_max_count(0);
        _add_snapshot(snapshot.clone());
        let result = SNAPSHOTS.with(|snapshots| snapshots.borrow().get(&snapshot.id));
        assert_eq!(result, None);
        _update_max_count(1);
        let new_snapshot_1 = Snapshot {
            id: SnapshotId {
                id: "01HX8MP06M000000000000004F".to_string(),
            },
            value: HashMap::new(),
            scores: HashMap::new(),
        };
        let new_snapshot_2 = Snapshot {
            id: SnapshotId {
                id: "01HX8MP06M000000000000004F".to_string(),
            },
            value: HashMap::new(),
            scores: HashMap::new(),
        };
        _add_snapshot(new_snapshot_1.clone());
        _add_snapshot(new_snapshot_2.clone());
        let result_1 = SNAPSHOTS.with(|snapshots| snapshots.borrow().get(&new_snapshot_1.id));
        assert_eq!(result_1, None);
        let result_2 = SNAPSHOTS.with(|snapshots| snapshots.borrow().get(&new_snapshot_2.id));
        assert_eq!(result_2.unwrap().id, new_snapshot_2.id);
        assert_eq!(snapshots_len(), 1);
    }
}

did_export!("rating_indexer");
