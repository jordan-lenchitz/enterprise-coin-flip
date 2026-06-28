use rand::Rng;
use tracing::instrument;

#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }
    
    pub fn mag_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

pub struct QuantumState {
    pub amplitudes: [Complex; 2],
}

impl QuantumState {
    #[instrument(level = "debug")]
    pub fn new_zero_state() -> Self {
        tracing::debug!("Initializing |0> state vector");
        QuantumState {
            amplitudes: [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub fn apply_hadamard(&mut self) {
        tracing::debug!("Applying Hadamard gate to state vector");
        let inv_sqrt2 = 1.0 / std::f64::consts::SQRT_2;
        let a0 = self.amplitudes[0];
        let a1 = self.amplitudes[1];

        self.amplitudes[0] = Complex::new(
            inv_sqrt2 * (a0.re + a1.re),
            inv_sqrt2 * (a0.im + a1.im)
        );
        self.amplitudes[1] = Complex::new(
            inv_sqrt2 * (a0.re - a1.re),
            inv_sqrt2 * (a0.im - a1.im)
        );
    }

    #[instrument(level = "debug", skip(self))]
    pub fn measure(&self) -> u8 {
        tracing::debug!("Measuring state vector");
        let prob_0 = self.amplitudes[0].mag_sq();
        let mut rng = rand::thread_rng();
        let sample: f64 = rng.r#gen();
        if sample < prob_0 {
            tracing::debug!("Measurement collapsed to 0 (HEADS)");
            0
        } else {
            tracing::debug!("Measurement collapsed to 1 (TAILS)");
            1
        }
    }
}

#[instrument(name = "simulate_coin_flip_pure_rust")]
pub fn simulate_coin_flip() -> u8 {
    let mut state = QuantumState::new_zero_state();
    state.apply_hadamard();
    state.measure()
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
