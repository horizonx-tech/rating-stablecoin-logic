use std::{collections::HashMap, str::FromStr};

use candid::Principal;
use common::{calc, Args, CalculateInput};
use variance_accessors::*;
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
            .map(|(k, v)| (k.clone(), score_variance(v, &values_all_assets)))
            .collect();
        LensValue { value }
    }
}

pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let target = Principal::from_str(&targets[0]).unwrap();
    calc(target, args).await.unwrap()
}

fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let sum: f64 = data.iter().sum();
    let n = data.len() as f64;

    sum / n
}

fn variance(data: &[f64]) -> f64 {
    let data_mean = mean(data);
    let n = data.len() as f64;

    if n == 0.0 {
        return 0.0;
    }

    let variance = data
        .iter()
        .map(|&x| {
            let diff = x - data_mean;
            diff * diff
        })
        .sum::<f64>()
        / n;
    variance
}

fn negative_log10_variance(data: &[f64]) -> f64 {
    let variance = variance(data);
    if variance == 0.0 {
        0.0
    } else {
        -variance.log10()
    }
}

fn max_negative_log10_variance(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| negative_log10_variance(data))
        .fold(0.0, f64::max)
}

fn score_variance(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_variance = negative_log10_variance(data);
    let max_log10_variance = max_negative_log10_variance(datasets);

    if max_log10_variance == 0.0 {
        0.0
    } else {
        log10_variance / max_log10_variance
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
        let result = variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 0.0;
        let result = variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_1week() {
        let data = usdc;
        let expected = 0.0000006272696734693033;
        let result = variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_1week() {
        let data = usdt;
        let expected = 0.0000001415482448979294;
        let result = variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 6.202545708737672;
        let result = negative_log10_variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 6.849095511222015;
        let result = negative_log10_variance(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.9056007028336818;
        let result = score_variance(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_variance(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_dai() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8870193377333394;
        let result = score_variance(&dai, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.7680064070586001;
        let result = score_variance(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
