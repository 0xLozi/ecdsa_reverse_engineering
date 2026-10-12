use num_bigint::BigUint;

// mod addition here
pub fn mod_add(a: &BigUint, b: &BigUint, p: &BigUint) -> BigUint {
    (a + b) % p
}

// module substraction here
pub fn mod_sub(a: &BigUint, b: &BigUint, p: &BigUint) -> BigUint {
    (a + p - (b % p)) % p
}

// module inversion here
pub fn mod_inv(a: &BigUint, p: &BigUint) -> BigUint {
    let two = BigUint::from(2u32);
    a.modpow(&(p - &two), p)
}

pub fn mod_square(a: &BigUint, p: &BigUint) -> BigUint {
    (a * a) % p
}

pub fn mod_mul(a: &BigUint, b: &BigUint, p: &BigUint) -> BigUint {
    (a * b) % p
}