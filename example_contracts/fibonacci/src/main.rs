#![no_main]

struct Contract;

#[multivm_sdk_macros::contract]
impl Contract {
    pub fn fibonacci(n: u32) {
        let (mut a, mut b) = (0u64, 1);
        for _ in 0..n {
            let c = a;
            a = b;
            b += c;
        }
        env::commit(a);
    }
}
