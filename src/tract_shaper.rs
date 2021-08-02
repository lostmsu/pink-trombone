use std::f64::consts::PI;

use crate::{math, tract::Tract, transient::Transient};

pub struct TractShaper {
    pub tract: Tract,
    velum_open_target: f32,
    velum_closed_target: f32,
    target_diameter: [f64; Tract::N],
    velum_target: f32,
    pub tongue_index: f64,
    pub tongue_diameter: f64,
    last_obstruction: i32,
}

const GRID_OFFSET: f64 = 1.7;
const MOVEMENT_SPEED: f64 = 15.0;

impl TractShaper {
    pub fn new(tract: Tract) -> TractShaper {
        let mut res = TractShaper {
            tract,
            velum_open_target: 0.4,
            velum_closed_target: 0.01,
            velum_target: 0.0,
            tongue_index: 12.9,
            tongue_diameter: 2.43,
            last_obstruction: -1,
            target_diameter: [0.0; Tract::N],
        };
        res.shape_noise(true);
        res.tract.calculate_nose_reflections(); // (nose reflections are calculated only once, but with open velum)
        res.shape_noise(false);
        res.shape_main_tract();
        res
    }

    fn shape_main_tract(&mut self) {
        for i in 0..Tract::N {
            let d = self.get_rest_diameter(i);
            self.tract.diameter[i] = d;
            self.target_diameter[i] = d;
        }
    }

    pub fn get_rest_diameter(&self, i: usize) -> f64 {
        if i < 7 {
            return 0.6;
        }
        if i < Tract::BLADE_START {
            return 1.1;
        }
        if i >= Tract::LIP_START {
            return 1.5;
        }

        let t = 1.1 * PI * (self.tongue_index - i as f64)
            / (Tract::TIP_START - Tract::BLADE_START) as f64;
        let fixed_tongue_diameter = 2.0 + (self.tongue_diameter - 2.0) / 1.5;
        let mut curve = (1.5 - fixed_tongue_diameter + GRID_OFFSET) * t.cos();

        if i == Tract::BLADE_START - 2 || i == Tract::LIP_START - 1 {
            curve *= 0.8;
        }
        if i == Tract::BLADE_START || i == Tract::LIP_START - 2 {
            curve *= 0.94;
        }

        1.5 - curve
    }

    pub fn adjust_tract_shape(&mut self, delta_time: f64) {
        let amount = delta_time * MOVEMENT_SPEED;
        let mut new_last_obstruction: i32 = -1;
        for i in 0..Tract::N {
            let diameter = self.tract.diameter[i];
            let target_diameter = self.target_diameter[i];
            if diameter <= 0.0 {
                new_last_obstruction = i as i32;
            }
            let slow_return = if i < Tract::NOSE_START {
                0.6
            } else if i >= Tract::TIP_START {
                1.0
            } else {
                0.6 + 0.4 * (i - Tract::NOSE_START) as f64
                    / (Tract::TIP_START - Tract::NOSE_START) as f64
            };

            self.tract.diameter[i] = math::move_towards(
                diameter,
                target_diameter,
                slow_return * amount,
                2.0 * amount,
            );
        }

        if self.last_obstruction >= 0
            && new_last_obstruction < 0
            && self.tract.nose_diameter[0] < 0.223
        {
            self.add_transient(self.last_obstruction as usize);
        }
        self.last_obstruction = new_last_obstruction;
        self.tract.nose_diameter[0] = math::move_towards(
            self.tract.nose_diameter[0],
            self.velum_target as f64,
            amount * 0.25,
            amount * 0.1,
        );
    }

    fn add_transient(&mut self, position: usize) {
        self.tract.transients.push(Transient {
            position,
            start_time: self.tract.time,
            life_time: 0.2,
            strength: 0.3,
            exponent: 200.0,
        });
    }

    fn shape_noise(&mut self, velum_open: bool) {
        self.set_velum_open(velum_open);
        for i in 0..Tract::NOSE_LEN {
            let d = i as f64 * 2.0 / Tract::NOSE_LEN as f64;
            let mut diameter = if i == 0 {
                self.velum_target as f64
            } else if d < 1.0 {
                0.4 + 1.6 * d
            } else {
                0.5 + 1.5 * (2.0 - d)
            };

            diameter = diameter.min(1.9);
            self.tract.nose_diameter[i] = diameter;
        }
    }

    pub fn set_velum_open(&mut self, velum_open: bool) {
        self.velum_target = if velum_open {
            self.velum_open_target
        } else {
            self.velum_closed_target
        };
    }
}
