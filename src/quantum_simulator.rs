use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    #[inline]
    pub fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    #[inline]
    pub fn mag_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

pub struct QuantumState {
    pub amplitudes: [Complex; 2],
}

impl QuantumState {
    #[inline]
    pub fn new_zero_state() -> Self {
        QuantumState {
            amplitudes: [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        }
    }

    #[inline]
    pub fn apply_hadamard(&mut self) {
        let inv_sqrt2 = 1.0 / std::f64::consts::SQRT_2;
        let a0 = self.amplitudes[0];
        let a1 = self.amplitudes[1];

        self.amplitudes[0] = Complex::new(inv_sqrt2 * (a0.re + a1.re), inv_sqrt2 * (a0.im + a1.im));
        self.amplitudes[1] = Complex::new(inv_sqrt2 * (a0.re - a1.re), inv_sqrt2 * (a0.im - a1.im));
    }

    #[inline]
    pub fn measure_with_rng<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        let prob_0 = self.amplitudes[0].mag_sq();
        let sample: f64 = rng.r#gen();
        if sample < prob_0 {
            0
        } else {
            1
        }
    }

    #[inline]
    pub fn measure(&self) -> u8 {
        let mut rng = rand::thread_rng();
        self.measure_with_rng(&mut rng)
    }
}

#[inline]
pub fn simulate_coin_flip() -> u8 {
    let mut state = QuantumState::new_zero_state();
    state.apply_hadamard();
    state.measure()
}

/// Simulates `count` independent Hadamard coin flips while reusing the prepared
/// superposition probability and a single thread-local RNG handle.
pub fn simulate_coin_flips(count: usize) -> Vec<u8> {
    let mut state = QuantumState::new_zero_state();
    state.apply_hadamard();
    let prob_0 = state.amplitudes[0].mag_sq();
    let mut rng = rand::thread_rng();
    let mut results = Vec::with_capacity(count);
    for _ in 0..count {
        let sample: f64 = rng.r#gen();
        results.push(if sample < prob_0 { 0 } else { 1 });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_simulator_hadamard() {
        let mut state = QuantumState::new_zero_state();
        state.apply_hadamard();
        
        let inv_sqrt2 = 1.0 / std::f64::consts::SQRT_2;
        assert!((state.amplitudes[0].re - inv_sqrt2).abs() < 1e-6);
        assert!((state.amplitudes[0].im).abs() < 1e-6);
        assert!((state.amplitudes[1].re - inv_sqrt2).abs() < 1e-6);
        assert!((state.amplitudes[1].im).abs() < 1e-6);
    }

    #[test]
    fn test_complex_mag_sq() {
        let c = Complex::new(3.0, 4.0);
        assert_eq!(c.mag_sq(), 25.0);
    }
}
