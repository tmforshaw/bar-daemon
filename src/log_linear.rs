#[must_use]
pub fn linear_to_logarithmic(linear_percent: f64) -> f64 {
    if linear_percent <= 0.0 {
        return 0.0;
    }

    if linear_percent >= 100.0 {
        return 100.0;
    }

    50.0 * (linear_percent / 100.0).mul_add(99.0, 1.0).log10()
}

#[must_use]
pub fn logarithmic_to_linear(log_percent: f64) -> f64 {
    if log_percent <= 0.0 {
        return 0.0;
    }
    if log_percent >= 100.0 {
        return 100.0;
    }

    let power = log_percent / 50.0;
    (10_f64.powf(power) - 1.0) / 99.0 * 100.0
}
