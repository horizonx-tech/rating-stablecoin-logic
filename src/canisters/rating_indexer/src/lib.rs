use std::cell::RefCell;

use candid::{Decode, Encode, Principal};
use chainsight_cdk::rpc::Caller;
use chainsight_cdk::rpc::Receiver;
use chainsight_cdk::rpc::ReceiverProvider;
use chainsight_cdk::rpc::ReceiverProviderWithoutArgs;
use chainsight_cdk::rpc::{CallProvider, Message};
use chainsight_cdk_macros::{
    chainsight_common, did_export, init_in, prepare_stable_structure, stable_memory_for_scalar,
    timer_task_func, StableMemoryStorable,
};

use ic_cdk::api::management_canister::main::raw_rand;
use ic_stable_structures::{memory_manager::MemoryId, BTreeMap};
use ic_web3_rs::futures::future::join_all;
use rating_indexer::CallCanisterArgs;
use rating_indexer_bindings::{Args, LensArgs, LensValue, Sources};
use types::{
    CalculationStragety, Key, QueryOption, Snapshot, SnapshotId, SnapshotValue, Task, TaskArgs,
    Value,
};
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
fn test_add_task() {
    let task = Task {
        id: "test".to_string(),
        source: Principal::from_text("lpcxv-qqaaa-aaaao-a3mdq-cai").unwrap(),
        args: TaskArgs {
            id: "coingecko:bitcoin".to_string(),
            ids: vec!["coingecko:bitcoin".to_string()],
        },
        strategy: CalculationStragety { weight: 1.0 },
        lens: Principal::from_text("ndkhw-hyaaa-aaaao-a3mua-cai").unwrap(), // dex liquidity
    };
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
            id: "".to_string(),
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

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn query_between(opt: QueryOption) -> Vec<Snapshot> {
    let from = opt.from_timestamp.unwrap_or(0);
    let to = opt.to_timestamp.unwrap_or(0);
    let divisor = 1_000_000; // nanosec to msec
    snapshots_between(from / divisor, to / divisor).await
}

async fn snapshots_between(from: i64, to: i64) -> Vec<Snapshot> {
    let mut result = vec![];
    let rand = raw_rand().await.unwrap().0;
    let from_id = Ulid::from_parts(from as u64, u128::from(rand[0]));
    let to_id = Ulid::from_parts(to as u64, u128::from(rand[0]));
    let mut ids_to_fetch = vec![];
    SNAPSHOT_IDS.with(|id| {
        let snapshot_len = id.borrow().len();
        for idx in 0..snapshot_len {
            let id = id.borrow().get(snapshot_len - idx - 1).unwrap();
            let ulid = Ulid::from_string(&id.id).unwrap();
            if ulid.lt(&from_id) {
                break;
            }
            if ulid.le(&to_id) {
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

async fn _index() -> Result<(), String> {
    let mut futures = vec![];
    let mut strategies = vec![];
    let mut ids = vec![];
    let duration = duration_seconds();
    TASKS.with(|tasks| {
        for task in tasks.borrow().iter() {
            let future = call(task.1.lens, task.1.to_lens_args(duration));
            futures.push(future);
            strategies.push(task.1.strategy);
            ids.push(task.0.clone());
        }
    });
    let results: Vec<(f64, CalculationStragety)> = join_all(futures)
        .await
        .iter()
        .zip(strategies.iter())
        .map(|(r, s)| (r.clone().unwrap(), s.clone()))
        .collect();
    let score_input: Vec<(f64, Option<f64>)> = results
        .into_iter()
        .map(|(score, strategy)| (score, Some(strategy.weight)))
        .collect();
    let scores = ids
        .into_iter()
        .zip(score_input.clone().into_iter())
        .map(|(id, score_input)| (id.id, score_input.0))
        .collect();
    let score = ScoreCalculator::new().calculate(score_input);
    let new_snapshot = Snapshot {
        id: SnapshotId::new().await,
        value: score,
        scores,
    };
    _add_snapshot(new_snapshot);
    Ok(())
}

#[ic_cdk::update]
#[candid::candid_method(update)]
async fn index() {
    only_proxy!();
    _index().await.unwrap();
}

async fn call(lens: Principal, args: LensArgs) -> Result<f64, String> {
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
        .reply::<LensValue>()
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
            value: 0.0,
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
            value: 0.0,
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
            value: 0.0,
            scores: HashMap::new(),
        };
        let new_snapshot_2 = Snapshot {
            id: SnapshotId {
                id: "01HX8MP06M000000000000004E".to_string(),
            },
            value: 0.0,
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
