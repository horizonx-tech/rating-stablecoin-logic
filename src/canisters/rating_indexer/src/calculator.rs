pub struct ScoreCalculator;

impl ScoreCalculator {
    pub fn new() -> Self {
        ScoreCalculator
    }

    pub fn calculate(&self, scores: Vec<(f64, Option<f64>)>) -> f64 {
        rating(scores)
    }
}
fn rating(scores: Vec<(f64, Option<f64>)>) -> f64 {
    let size = scores.len() as f64;
    scores
        .iter()
        .map(|s| s.0.powf(s.1.unwrap_or(1.0) / size))
        .reduce(|a, b| a * b)
        .unwrap_or(0.0)
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
        let result = rating(vec![
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_all_elements_same() {
        let expected = 1.0;
        let result = rating(vec![
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
            (0.0, None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdc_rating() {
        let expected = 4.017856419662701;
        let result = rating(vec![
            (usdc[0], None),
            (usdc[1], None),
            (usdc[2], None),
            (usdc[3], None),
            (usdc[4], None),
            (usdc[5], None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_usdt_rating() {
        let expected = 4.8653063858730174;
        let result = rating(vec![
            (usdt[0], None),
            (usdt[1], None),
            (usdt[2], None),
            (usdt[3], None),
            (usdt[4], None),
            (usdt[5], None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_dai_rating() {
        let expected = 4.3798898528148325;
        let result = rating(vec![
            (dai[0], None),
            (dai[1], None),
            (dai[2], None),
            (dai[3], None),
            (dai[4], None),
            (dai[5], None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_fdusd_rating() {
        let expected = 3.4510228498125803;
        let result = rating(vec![
            (fdusd[0], None),
            (fdusd[1], None),
            (fdusd[2], None),
            (fdusd[3], None),
            (fdusd[4], None),
            (fdusd[5], None),
        ]);
        assert_eq!(result, expected, "Expected {}, got {}", expected, result);
    }
}
