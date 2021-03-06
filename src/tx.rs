use std::collections::HashMap;

use protobuf::Message;

use crypto::{PublicKey, SecretKey, Signature, sign, verify_signature};
use ironcoin_pb::{Commitment, DetachedSignature, Transaction, Transfer};
use error::{IroncError, IroncResult};

pub trait TransactionExt {
    fn verify_signatures(&self) -> IroncResult<()>;
}

impl TransactionExt for Transaction {
    fn verify_signatures(&self) -> IroncResult<()> {
        let commit_bytes = &try!(self.get_commit().write_to_bytes());
        let mut sign_map = HashMap::<&[u8], &[u8]>::new();
        for sign in self.get_signatures().iter() {
            sign_map.insert(sign.get_public_key(), sign.get_payload());
        }
        for transfer in self.get_commit().get_transfers().iter() {
            match sign_map.get(transfer.get_source_pk()) {
                Some(sign_bytes) => {
                    let public_key =
                        try!(PublicKey::from_slice(transfer.get_source_pk()));
                    let signature = try!(Signature::from_slice(sign_bytes));
                    try!(verify_signature(&public_key, commit_bytes, &signature));
                },
                None => return Err(IroncError::new("Missing key."))
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct TransactionBuilder {
    transfer_secret_keys: Vec<SecretKey>,
    bounty_secret_key: Option<SecretKey>,
    commit: Commitment
}

impl TransactionBuilder {
    pub fn new() -> TransactionBuilder {
        TransactionBuilder {
            transfer_secret_keys: Vec::<SecretKey>::new(),
            bounty_secret_key: None,
            commit: Commitment::new()
        }
    }

    pub fn add_transfer(
        &mut self, sk: &SecretKey, source: &PublicKey, destination: &PublicKey,
        tokens: u64, op_index:u32) -> &mut Self {
        let mut transfer = Transfer::new();
        transfer.set_op_index(op_index);
        transfer.set_tokens(tokens);
        transfer.mut_source_pk().push_all(&source.0);
        transfer.mut_destination_pk().push_all(&destination.0);

        self.transfer_secret_keys.push(sk.clone());
        self.commit.mut_transfers().push(transfer);
        self
    }

    pub fn set_bounty(&mut self, sk: &SecretKey, source: &PublicKey,
                      bounty: u64) -> &mut Self {
        self.bounty_secret_key = Some(sk.clone());
        self.commit.mut_bounty_pk().push_all(&source.0);
        self.commit.set_bounty(bounty);
        self
    }

    pub fn build(self) -> IroncResult<Transaction> {
        let mut transaction = Transaction::new();
        let commit_bytes = &self.commit.write_to_bytes().unwrap();
        for (transfer, secret_key) in self.commit.get_transfers().iter()
            .zip(self.transfer_secret_keys.iter())
        {
            let signature = sign(secret_key, commit_bytes);
            let pk = try!(PublicKey::from_slice(transfer.get_source_pk()));
            match verify_signature(&pk, commit_bytes, &signature) {
                Ok(_) => {
                    let mut sign = DetachedSignature::new();
                    sign.set_public_key(pk.0.to_vec());
                    sign.set_payload(signature.0.to_vec());
                    transaction.mut_signatures().push(sign);
                },
                Err(_) => return Err(
                    IroncError::new("Invalid key for source account."))
            }
        }
        transaction.set_commit(self.commit);
        try!(transaction.verify_signatures());
        Ok(transaction)
    }
}
