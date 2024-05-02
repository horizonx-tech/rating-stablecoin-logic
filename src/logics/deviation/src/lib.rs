use std::str::FromStr;

use candid::{CandidType, Principal};
use indexer::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub value: f64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, Default)]
pub struct CalculateArgs {
    id: String,
    ids: Vec<String>,
    from: i64,
    to: i64,
}
pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let indexer =
        BulkSnapshotIndexerHttps::new(Principal::from_str(targets.get(0).unwrap()).unwrap());
    let value = indexer.query(args.id, args.from, args.to).await.unwrap();
    let values = value
        .iter()
        .map(|x| x.value().unwrap())
        .collect::<Vec<f64>>();
    let mut value_all_assets = vec![];
    for id in args.ids {
        let value = indexer.query(id, args.from, args.to).await.unwrap();
        let values = value
            .iter()
            .map(|x| x.value().unwrap())
            .collect::<Vec<f64>>();
        value_all_assets.push(values);
    }
    LensValue {
        value: score_deviation(&values, &value_all_assets),
    }
}

fn average_deviation(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let deviation = data.iter().map(|&x| (x - 1.0).abs()).sum::<f64>();
    deviation / n
}

fn negative_log10_deviation(data: &[f64]) -> f64 {
    let deviation = average_deviation(data);
    if deviation == 0.0 {
        0.0
    } else {
        -deviation.log10()
    }
}

fn max_negative_log10_deviation(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| negative_log10_deviation(data))
        .fold(0.0, f64::max)
}

fn score_deviation(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_deviation = negative_log10_deviation(data);
    let max_log10_deviation = max_negative_log10_deviation(datasets);

    if max_log10_deviation == 0.0 {
        0.0
    } else {
        log10_deviation / max_log10_deviation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const usdc: [f64; 7] = [
        0.999482, 1.001000, 0.999570, 1.001000, 1.001000, 0.998959, 1.000000,
    ];
    const usdt: [f64; 7] = [
        1.000000, 0.999738, 1.000000, 1.000000, 1.000000, 1.000000, 1.001000,
    ];
    const dai: [f64; 7] = [
        0.998615, 1.000000, 1.000000, 0.999913, 1.000000, 1.002000, 1.000000,
    ];
    const fdusd: [f64; 7] = [
        0.997341, 1.000000, 1.000000, 1.002000, 1.005000, 0.998214, 1.001000,
    ];

    #[test]
    fn test_empty_slice() {
        let data = [];
        let expected = 0.0;
        let result = average_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 0.0;
        let result = average_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_1week() {
        let data = usdc;
        let expected = 0.0007127142857142411;
        let result = average_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_1week() {
        let data = usdt;
        let expected = 0.00018028571428569634;
        let result = average_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 3.1470845360751025;
        let result = negative_log10_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 3.7440386851061844;
        let result = negative_log10_deviation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8405587657505329;
        let result = score_deviation(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_deviation(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_dai() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8826079539870174;
        let result = score_deviation(&dai, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.734528505276825;
        let result = score_deviation(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
