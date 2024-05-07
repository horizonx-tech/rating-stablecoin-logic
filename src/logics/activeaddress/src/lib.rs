use std::str::FromStr;
pub type CalculateArgs = Args;
use activeaddress_accessors::*;
use candid::Principal;
use common::call_with_transform;
use common::Args;
use common::CalculateInput;
#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub value: f64,
}
impl From<CalculateInput> for LensValue {
    fn from(input: CalculateInput) -> Self {
        let value = input.values;
        let value_all_assets = input.value_all_assets;
        let score = score_address(&value, &value_all_assets);
        LensValue { value: score }
    }
}

pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let target = Principal::from_str(&targets[0]).unwrap();

    let v = call_with_transform(target, args, |f| f.value_from_string().unwrap())
        .await
        .unwrap();
    LensValue::from(v)
}
fn average_address(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let sum: f64 = data.iter().sum();
    sum / n
}

fn log10_address(data: &[f64]) -> f64 {
    let average = average_address(data);
    average.log10()
}

fn max_log10_address(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| log10_address(data))
        .fold(0.0, f64::max)
}

fn score_address(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_address = log10_address(data);
    let max_log10_address = max_log10_address(datasets);

    if max_log10_address == 0.0 {
        0.0
    } else {
        log10_address / max_log10_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const usdc: [f64; 6] = [20756.0, 39127.0, 20996.0, 20644.0, 21952.0, 24694.0];
    const usdt: [f64; 6] = [55211.0, 96979.0, 50291.0, 51362.0, 49945.0, 51539.0];
    const dai: [f64; 6] = [1615.0, 2625.0, 1476.0, 1849.0, 2057.0, 2399.0];
    const fdusd: [f64; 6] = [26.0, 48.0, 31.0, 76.0, 31.0, 22.0];

    #[test]
    fn test_empty_slice() {
        let data = [];
        let expected = 0.0;
        let result = average_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 1.0;
        let result = average_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_6days() {
        let data = usdc;
        let expected = 24694.833333333332;
        let result = average_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_6days() {
        let data = usdt;
        let expected = 59221.166666666664;
        let result = average_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 4.392606099432254;
        let result = log10_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 4.772476958809861;
        let result = log10_address(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.9204038358579446;
        let result = score_address(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_address(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_dai() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.6918397669104942;
        let result = score_address(&dai, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.33338340253051146;
        let result = score_address(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
