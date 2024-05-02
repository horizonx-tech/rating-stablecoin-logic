use std::str::FromStr;

use candid::Principal;
use common::{calc, Args, CalculateInput};
pub type CalculateArgs = Args;
#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub value: f64,
}
impl From<CalculateInput> for LensValue {
    fn from(input: CalculateInput) -> Self {
        let value = input.values;
        let value_all_assets = input.value_all_assets;
        let score = score_liquidity(&value, &value_all_assets);
        LensValue { value: score }
    }
}

pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let target = Principal::from_str(&targets[0]).unwrap();
    calc(target, args).await.unwrap()
}

fn average_liquidity(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let sum: f64 = data.iter().sum();
    sum / n
}

fn log10_liquidity(data: &[f64]) -> f64 {
    let average = average_liquidity(data);
    average.log10()
}

fn max_log10_liquidity(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| log10_liquidity(data))
        .fold(0.0, f64::max)
}

fn score_liquidity(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_liquidity = log10_liquidity(data);
    let max_log10_liquidity = max_log10_liquidity(datasets);

    if max_log10_liquidity == 0.0 {
        0.0
    } else {
        log10_liquidity / max_log10_liquidity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const usdc: [f64; 9] = [
        92852142.73,
        96521991.91,
        104605045.15,
        67397382.39,
        72825403.63,
        71642046.15,
        65437707.89,
        64151262.60,
        43014497.08,
    ];
    const usdt: [f64; 9] = [
        2466954.95,
        20692204.12,
        2279293.81,
        9899416.37,
        1213691.66,
        476400.51,
        355816.98,
        13984077.51,
        229678.67,
    ];
    const dai: [f64; 9] = [
        5558588.81, 6043658.45, 5640429.57, 3072559.7, 3295090.65, 1683993.6, 1599624.51,
        1644650.53, 7111106.29,
    ];
    const fdusd: [f64; 9] = [
        370916.93, 339327.94, 330683.76, 340056.62, 338320.95, 350043.94, 349716.82, 370261.87,
        367124.08,
    ];

    #[test]
    fn test_empty_slice() {
        let data = [];
        let expected = 0.0;
        let result = average_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 1.0;
        let result = average_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_9days() {
        let data = usdc;
        let expected = 75383053.2811111;
        let result = average_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_9days() {
        let data = usdt;
        let expected = 5733059.397777777;
        let result = average_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 7.877273723937194;
        let result = log10_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 6.758386441337469;
        let result = log10_liquidity(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_liquidity(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8579600859622678;
        let result = score_liquidity(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_dai() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.8375757434626842;
        let result = score_liquidity(&dai, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.7039182589704108;
        let result = score_liquidity(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
