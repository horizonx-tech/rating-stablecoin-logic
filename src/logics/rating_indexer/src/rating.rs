#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct LensValue {
    pub dummy: u64,
}
#[derive(Clone, Debug, Default, candid :: CandidType, serde :: Deserialize, serde :: Serialize)]
pub struct CalculateArgs {
    pub dummy: u64,
}
pub async fn calculate(targets: Vec<String>, args: CalculateArgs) -> LensValue {
    todo!()
}

fn rating(
    score_avedev: f64,
    score_var: f64,
    scoreautcor: f64,
    score_dexliq: f64,
    score_address: f64,
    score_txvol: f64,
) -> f64 {
    let score_avedev = score_avedev.powf(1.0 / 6.0);
    let score_var = score_var.powf(1.0 / 6.0);
    let score_autcor = scoreautcor.powf(1.0 / 6.0);
    let score_dexliq = score_dexliq.powf(1.0 / 6.0);
    let score_address = score_address.powf(1.0 / 6.0);
    let score_txvol = score_txvol.powf(1.0 / 6.0);

    score_avedev * score_var * score_autcor * score_dexliq * score_address * score_txvol
}

#[cfg(test)]
mod tests {
    use super::*;

    const usdc: [f64; 6] = [4.202794, 1.921468, 4.528004, 5.000000, 4.602019, 5.000000];
    const usdt: [f64; 6] = [5.000000, 5.000000, 5.000000, 4.289800, 5.000000, 4.947025];
    const dai: [f64; 6] = [4.413040, 5.000000, 4.435097, 4.187879, 3.459199, 4.979646];
    const fdusd: [f64; 6] = [3.672643, 4.644562, 3.840032, 3.519591, 1.666917, 4.395644];

    #[test]
    fn test_empty_slice() {
        let expected = 0.0;
        let result = rating(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let expected = 1.0;
        let result = rating(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_rating() {
        let expected = 4.017856419662701;
        let result = rating(usdc[0], usdc[1], usdc[2], usdc[3], usdc[4], usdc[5]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_rating() {
        let expected = 4.8653063858730174;
        let result = rating(usdt[0], usdt[1], usdt[2], usdt[3], usdt[4], usdt[5]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_dai_rating() {
        let expected = 4.3798898528148325;
        let result = rating(dai[0], dai[1], dai[2], dai[3], dai[4], dai[5]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_fdusd_rating() {
        let expected = 3.4510228498125803;
        let result = rating(fdusd[0], fdusd[1], fdusd[2], fdusd[3], fdusd[4], fdusd[5]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
