use bellman::groth16;
use bls12_381::Bls12;
use ff::{Field, PrimeField};
use group::Group;
use rand::rngs::OsRng;
use std::io;

use sapvi::crypto::{
    coin::Coin,
    create_mint_proof, create_spend_proof, load_params,
    merkle::{CommitmentTree, IncrementalWitness},
    note::Note,
    save_params, setup_mint_prover, setup_spend_prover, verify_mint_proof, verify_spend_proof,
    MintRevealedValues, SpendRevealedValues,
};
use sapvi::error::{Error, Result};
use sapvi::serial::{Decodable, Encodable, VarInt};
use sapvi::tx;

fn txbuilding() {
    {
        let params = setup_mint_prover();
        save_params("mint.params", &params);
    }
    {
        let params = setup_spend_prover();
        save_params("spend.params", &params);
    }
    let (mint_params, mint_pvk) = load_params("mint.params").expect("params should load");
    let (spend_params, spend_pvk) = load_params("spend.params").expect("params should load");

    let public = jubjub::SubgroupPoint::random(&mut OsRng);

    let builder = tx::TransactionBuilder {
        clear_inputs: vec![tx::TransactionBuilderClearInputInfo { value: 110 }],
        outputs: vec![tx::TransactionBuilderOutputInfo { value: 110, public }],
    };

    let mut tx_data = vec![];
    {
        let tx = builder.build(&mint_params, &spend_params);
        tx.encode(&mut tx_data).expect("encode tx");
    }
    let mut tree = CommitmentTree::empty();
    for i in 0..5 {
        let cmu = Coin::new(bls12_381::Scalar::random(&mut OsRng).to_repr());
        tree.append(cmu);
    }
    {
        let tx = tx::Transaction::decode(&tx_data[..]).unwrap();
        assert!(tx.verify(&mint_pvk));
        tree.append(Coin::new(tx.outputs[0].revealed.coin))
            .expect("append merkle");
    }
    let mut witness = IncrementalWitness::from_tree(&tree);
    assert_eq!(witness.position(), 5);
    assert_eq!(tree.root(), witness.root());

    // Add some random coins in
    for i in 0..10 {
        let cmu = Coin::new(bls12_381::Scalar::random(&mut OsRng).to_repr());
        tree.append(cmu);
        witness.append(cmu);
        assert_eq!(tree.root(), witness.root());
    }

    let merkle_path = witness.path().unwrap();
    let auth_path: Vec<Option<(bls12_381::Scalar, bool)>> = merkle_path
        .auth_path
        .iter()
        .map(|(node, b)| Some(((*node).into(), *b)))
        .collect();

    /*let note = Note {
        serial: jubjub::Fr::random(&mut OsRng),
        value: 110,
        coin_blind: jubjub::Fr::random(&mut OsRng),
        valcom_blind: jubjub::Fr::random(&mut OsRng),
    };

    let secret = jubjub::Fr::random(&mut OsRng);
    let public = zcash_primitives::constants::SPENDING_KEY_GENERATOR * secret;

    let encrypted_note = note.encrypt(&public).unwrap();
    let note2 = encrypted_note.decrypt(&secret).unwrap();
    assert_eq!(note.value, note2.value);*/
}

fn main() {
    txbuilding();
}
