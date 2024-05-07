use std::{f64::consts::LOG10_E, str::FromStr};

use autocorrelation_accessors::*;
use candid::Principal;
use common::{call_and_score, Args};
pub type CalculateArgs = Args;
pub type LensValue = common::LensValue;

pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    let target = Principal::from_str(&targets[0]).unwrap();
    call_and_score(target, args, score_autocorrelation)
        .await
        .unwrap()
}

fn autocorrelation(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }

    let numerator: f64 = data.windows(2).map(|w| (w[1] - 1.0) * (w[0] - 1.0)).sum();
    let denominator: f64 = data
        .iter()
        .map(|&x| {
            let deviation = x - 1.0;
            deviation * deviation
        })
        .sum();

    if denominator == 0.0 {
        0.0
    } else {
        (numerator / denominator).abs()
    }
}

fn negative_log10_autocorrelation(data: &[f64]) -> f64 {
    let autocorrelation = autocorrelation(data);
    -(autocorrelation + 0.1).ln() * LOG10_E
}

fn max_negative_log10_autocorrelation(datasets: &[Vec<f64>]) -> f64 {
    datasets
        .iter()
        .map(|data| negative_log10_autocorrelation(data))
        .fold(0.0, f64::max)
}

fn score_autocorrelation(data: &[f64], datasets: &[Vec<f64>]) -> f64 {
    let log10_deviation = negative_log10_autocorrelation(data);
    let max_log10_deviation = max_negative_log10_autocorrelation(datasets);

    log10_deviation / max_log10_deviation
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
        let result = autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let data = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = 0.0;
        let result = autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_1week() {
        let data = usdc;
        let expected = 0.3127682858689416;
        let result = autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_1week() {
        let data = usdt;
        let expected = 0.0;
        let result = autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_fdusd_1week() {
        let data = fdusd;
        let expected = 0.017784367377130735;
        let result = autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_log10() {
        let data = usdc;
        let expected = 0.3842936781473731;
        let result = negative_log10_autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_log10() {
        let data = usdt;
        let expected = 0.9999999999999999;
        let result = negative_log10_autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_fdusd_log10() {
        let data = fdusd;
        let expected = 0.9289123463262241;
        let result = negative_log10_autocorrelation(&data);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdc() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.38429367814737314;
        let result = score_autocorrelation(&usdc, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_usdt() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 1.0;
        let result = score_autocorrelation(&usdt, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_score_fdusd() {
        let datasets = vec![usdc.to_vec(), usdt.to_vec(), dai.to_vec(), fdusd.to_vec()];
        let expected = 0.9289123463262242;
        let result = score_autocorrelation(&fdusd, &datasets);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
