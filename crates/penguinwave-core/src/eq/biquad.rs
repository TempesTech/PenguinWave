//! Biquad coefficient math — RBJ Audio-EQ-Cookbook formulas.
//! Coefficients are normalized by a0 to match PipeWire's `bq_raw` ports
//! (b0 b1 b2 a1 a2, with a0 assumed 1).

use penguinwave_proto::{EqBand, FilterType};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

pub const IDENTITY: Biquad = Biquad {
    b0: 1.0,
    b1: 0.0,
    b2: 0.0,
    a1: 0.0,
    a2: 0.0,
};

/// Gain-only biquad (used for the preamp node).
pub fn gain(db: f32) -> Biquad {
    Biquad {
        b0: 10f32.powf(db / 20.0),
        ..IDENTITY
    }
}

pub fn coefficients(band: &EqBand, sample_rate: f32) -> Biquad {
    if !band.enabled {
        return IDENTITY;
    }

    let a = 10f32.powf(band.gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * band.freq / sample_rate;
    let (sin_w0, cos_w0) = w0.sin_cos();
    let alpha = sin_w0 / (2.0 * band.q);

    let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
        FilterType::Peaking => (
            1.0 + alpha * a,
            -2.0 * cos_w0,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos_w0,
            1.0 - alpha / a,
        ),
        FilterType::LowShelf => {
            let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
            (
                a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
            )
        }
        FilterType::HighShelf => {
            let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
            (
                a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
            )
        }
        FilterType::LowPass => (
            (1.0 - cos_w0) / 2.0,
            1.0 - cos_w0,
            (1.0 - cos_w0) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        ),
        FilterType::HighPass => (
            (1.0 + cos_w0) / 2.0,
            -(1.0 + cos_w0),
            (1.0 + cos_w0) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        ),
        FilterType::Notch => (
            1.0,
            -2.0 * cos_w0,
            1.0,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        ),
    };

    Biquad {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Magnitude response |H(f)| in dB at a given frequency — used by tests
/// (the TS twin in src/lib/biquad.ts renders the UI curve from the same math).
pub fn magnitude_db(bq: &Biquad, freq: f32, sample_rate: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let (sin_w, cos_w) = w.sin_cos();
    let (sin_2w, cos_2w) = (2.0 * w).sin_cos();

    // |H(e^jw)|^2 = |b0 + b1 e^-jw + b2 e^-2jw|^2 / |1 + a1 e^-jw + a2 e^-2jw|^2
    let num_re = bq.b0 + bq.b1 * cos_w + bq.b2 * cos_2w;
    let num_im = -(bq.b1 * sin_w + bq.b2 * sin_2w);
    let den_re = 1.0 + bq.a1 * cos_w + bq.a2 * cos_2w;
    let den_im = -(bq.a1 * sin_w + bq.a2 * sin_2w);

    let num = num_re * num_re + num_im * num_im;
    let den = den_re * den_re + den_im * den_im;
    10.0 * (num / den).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use penguinwave_proto::{EqBand, FilterType};

    const SR: f32 = 48_000.0;

    fn band(filter_type: FilterType, freq: f32, gain_db: f32, q: f32) -> EqBand {
        EqBand {
            freq,
            gain_db,
            q,
            filter_type,
            enabled: true,
        }
    }

    #[test]
    fn disabled_band_is_identity() {
        let mut b = band(FilterType::Peaking, 1000.0, 6.0, 1.0);
        b.enabled = false;
        assert_eq!(coefficients(&b, SR), IDENTITY);
    }

    #[test]
    fn identity_is_transparent() {
        for freq in [20.0, 100.0, 1000.0, 10_000.0, 20_000.0] {
            assert!(magnitude_db(&IDENTITY, freq, SR).abs() < 1e-4);
        }
    }

    #[test]
    fn peaking_hits_target_gain_at_center() {
        for gain in [-12.0, -6.0, 3.0, 12.0] {
            let bq = coefficients(&band(FilterType::Peaking, 1000.0, gain, 1.1), SR);
            let mag = magnitude_db(&bq, 1000.0, SR);
            assert!((mag - gain).abs() < 0.05, "gain {gain}: got {mag}");
        }
    }

    #[test]
    fn peaking_is_flat_far_from_center() {
        let bq = coefficients(&band(FilterType::Peaking, 1000.0, 12.0, 1.1), SR);
        assert!(magnitude_db(&bq, 20.0, SR).abs() < 0.2);
        assert!(magnitude_db(&bq, 20_000.0, SR).abs() < 0.2);
    }

    #[test]
    fn low_shelf_boosts_lows_only() {
        let bq = coefficients(&band(FilterType::LowShelf, 200.0, 6.0, 0.707), SR);
        assert!((magnitude_db(&bq, 20.0, SR) - 6.0).abs() < 0.2);
        assert!(magnitude_db(&bq, 10_000.0, SR).abs() < 0.2);
    }

    #[test]
    fn high_shelf_boosts_highs_only() {
        let bq = coefficients(&band(FilterType::HighShelf, 5_000.0, 6.0, 0.707), SR);
        assert!((magnitude_db(&bq, 19_000.0, SR) - 6.0).abs() < 0.3);
        assert!(magnitude_db(&bq, 50.0, SR).abs() < 0.2);
    }

    #[test]
    fn lowpass_attenuates_highs() {
        let bq = coefficients(&band(FilterType::LowPass, 1000.0, 0.0, 0.707), SR);
        assert!(magnitude_db(&bq, 100.0, SR).abs() < 0.1);
        // -12 dB/octave: two octaves above cutoff ≈ -24 dB (Butterworth-ish)
        assert!(magnitude_db(&bq, 4000.0, SR) < -20.0);
    }

    #[test]
    fn highpass_attenuates_lows() {
        let bq = coefficients(&band(FilterType::HighPass, 1000.0, 0.0, 0.707), SR);
        assert!(magnitude_db(&bq, 10_000.0, SR).abs() < 0.1);
        assert!(magnitude_db(&bq, 250.0, SR) < -20.0);
    }

    #[test]
    fn notch_kills_center() {
        let bq = coefficients(&band(FilterType::Notch, 1000.0, 0.0, 2.0), SR);
        assert!(magnitude_db(&bq, 1000.0, SR) < -30.0);
        assert!(magnitude_db(&bq, 100.0, SR).abs() < 0.2);
    }

    #[test]
    fn preamp_gain_node() {
        let bq = gain(-6.0);
        assert!((magnitude_db(&bq, 1000.0, SR) + 6.0).abs() < 0.01);
    }
}
