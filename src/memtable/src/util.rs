
use rand::prelude::*;
use rand_distr::StandardGeometric;

pub fn generate_random_lvl(max_lvl: u64) -> u64 {
    let mut rng = rand::thread_rng();
    let num = StandardGeometric.sample(&mut rng);
    if num > max_lvl {
        max_lvl
    } else {
        num
    }
}
