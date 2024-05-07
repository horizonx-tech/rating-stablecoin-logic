use std::{collections::HashMap, str::FromStr};

use candid::Principal;
use common::{call_with_transform, Args, CalculateInput};
pub type CalculateArgs = Args;
#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub value: HashMap<String, f64>,
}
impl From<CalculateInput> for LensValue {
    fn from(input: CalculateInput) -> Self {
        let value_all_assets = input.value_all_assets;
        let values_all_assets: Vec<Vec<f64>> = value_all_assets.values().cloned().collect();
        let value = value_all_assets
            .iter()
            .map(|(k, v)| (k.clone(), score_volume(v, &values_all_assets)))
            .collect();
        LensValue { value }
    }
}

pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let target = Principal::from_str(&targets[0]).unwrap();

    let v = call_with_transform(target, args, |f| f.value_from_string().unwrap())
        .await
        .unwrap();
    LensValue::from(v)
}

fn average_volume(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let sum: f64 = data.iter().sum();
    sum / n
}

fn log10_volume(data: &[f64]) -> f64 {
    let average = average_volume(data);
    average.log10()
}

fn max_log10_volume(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| log10_volume(data))
        .fold(0.0, f64::max)
}

fn score_volume(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_volume = log10_volume(data);
    let max_log10_volume = max_log10_volume(datasets);

    if max_log10_volume == 0.0 {
        0.0
    } else {
        log10_volume / max_log10_volume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const usdc: [f64; 6] = [
        11297494841.0,
        16662147803.0,
        7903871311.0,
        14753101769.0,
        7527660368.0,
        9817033672.0,
    ];
    const usdt: [f64; 6] = [
        10595078509.0,
        12406512875.0,
        5989646841.0,
        9102603766.0,
        5563919565.0,
        9521087992.0,
    ];
    const dai: [f64; 6] = [
        11869832525.0,
        8445518282.0,
        4672267966.0,
        7374460791.0,
        10147684330.0,
        19339245814.0,
    ];
    const fdusd: [f64; 6] = [
        919638661.0,
        469195441.0,
        210240247.0,
        2153513563.0,
        386958583.0,
        544515.0,
    ];

    #[test]
    fn test_empty_slice() {
        let data = [];
        let expected = 0.0;
        let result = average_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 1.0;
        let result = average_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_6days() {
        let data = usdc;
        let expected = 11326884960.666666;
        let result = average_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_6days() {
        let data = usdt;
        let expected = 8863141591.333334;
        let result = average_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 10.054110489704426;
        let result = log10_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 9.947587687343765;
        let result = log10_volume(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_volume(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.9894050495596063;
        let result = score_volume(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_dai() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.9959291287089729;
        let result = score_volume(&dai, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8791288544937829;
        let result = score_volume(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
