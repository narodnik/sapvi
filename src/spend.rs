use bellman::gadgets::multipack;
use bellman::groth16;
use blake2s_simd::Params as Blake2sParams;
use bls12_381::Bls12;
use ff::Field;
use group::{Curve, Group, GroupEncoding};

mod spend_contract;
use spend_contract::SpendContract;

struct SpendRevealedValues {
    value_commit: jubjub::SubgroupPoint,
    nullifier: [u8; 32],
    coin: [u8; 32],
}

impl SpendRevealedValues {
    fn compute(
        value: u64,
        randomness_value: &jubjub::Fr,
        serial: &jubjub::Fr,
        randomness_coin: &jubjub::Fr,
        secret: &jubjub::Fr,
    ) -> Self {
        let value_commit = (zcash_primitives::constants::VALUE_COMMITMENT_VALUE_GENERATOR
            * jubjub::Fr::from(value))
            + (zcash_primitives::constants::VALUE_COMMITMENT_RANDOMNESS_GENERATOR
                * randomness_value);

        let mut nullifier = [0; 32];
        nullifier.copy_from_slice(
            Blake2sParams::new()
                .hash_length(32)
                .personal(zcash_primitives::constants::CRH_IVK_PERSONALIZATION)
                .to_state()
                .update(&secret.to_bytes())
                .update(&serial.to_bytes())
                .finalize()
                .as_bytes(),
        );

        let public = zcash_primitives::constants::SPENDING_KEY_GENERATOR * secret;

        let mut coin = [0; 32];
        coin.copy_from_slice(
            Blake2sParams::new()
                .hash_length(32)
                .personal(zcash_primitives::constants::CRH_IVK_PERSONALIZATION)
                .to_state()
                .update(&public.to_bytes())
                .update(&value.to_le_bytes())
                .update(&serial.to_bytes())
                .update(&randomness_coin.to_bytes())
                .finalize()
                .as_bytes(),
        );

        SpendRevealedValues { value_commit, nullifier, coin }
    }

    fn make_outputs(&self) -> [bls12_381::Scalar; 6] {
        let mut public_input = [bls12_381::Scalar::zero(); 6];

        {
            let result = jubjub::ExtendedPoint::from(self.value_commit);
            let affine = result.to_affine();
            //let (u, v) = (affine.get_u(), affine.get_v());
            let u = affine.get_u();
            let v = affine.get_v();
            public_input[0] = u;
            public_input[1] = v;
        }

        {
            // Pack the hash as inputs for proof verification.
            let hash = multipack::bytes_to_bits_le(&self.nullifier);
            let hash = multipack::compute_multipacking(&hash);

            // There are 2 chunks for a blake hash
            assert_eq!(hash.len(), 2);

            public_input[2] = hash[0];
            public_input[3] = hash[1];
        }

        {
            // Pack the hash as inputs for proof verification.
            let hash = multipack::bytes_to_bits_le(&self.coin);
            let hash = multipack::compute_multipacking(&hash);

            // There are 2 chunks for a blake hash
            assert_eq!(hash.len(), 2);

            public_input[4] = hash[0];
            public_input[5] = hash[1];
        }

        public_input
    }
}

fn main() {
    use std::time::Instant;
    use rand::rngs::OsRng;

    let value = 110;
    let randomness_value: jubjub::Fr = jubjub::Fr::random(&mut OsRng);

    let serial: jubjub::Fr = jubjub::Fr::random(&mut OsRng);
    let randomness_coin: jubjub::Fr = jubjub::Fr::random(&mut OsRng);
    let secret: jubjub::Fr = jubjub::Fr::random(&mut OsRng);

    let revealed =
        SpendRevealedValues::compute(value, &randomness_value, &serial, &randomness_coin, &secret);

    let start = Instant::now();
    let params = {
        let c = SpendContract {
            value: None,
            randomness_value: None,
            serial: None,
            randomness_coin: None,
            secret: None,
        };
        groth16::generate_random_parameters::<Bls12, _, _>(c, &mut OsRng).unwrap()
    };
    let pvk = groth16::prepare_verifying_key(&params.vk);
    println!("Setup: [{:?}]", start.elapsed());

    let c = SpendContract {
        value: Some(value),
        randomness_value: Some(randomness_value),
        serial: Some(serial),
        randomness_coin: Some(randomness_coin),
        secret: Some(secret),
    };

    let start = Instant::now();
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    println!("Prove: [{:?}]", start.elapsed());

    let public_input = revealed.make_outputs();

    let start = Instant::now();
    assert!(groth16::verify_proof(&pvk, &proof, &public_input).is_ok());
    println!("Verify: [{:?}]", start.elapsed());
}

