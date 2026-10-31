use crypto_bigint::{U256, Wrapping};

fn main() {
    println!("Testing crypto_bigint implementation");
    let a = Wrapping(U256::MAX);
    let b = Wrapping(U256::ONE);
    let c = a + b;

    assert_eq!(c.0, U256::ZERO);
}
