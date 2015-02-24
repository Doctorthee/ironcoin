use std::fs::File;
use std::io::{Read, Write};
use std::iter::count;
use std::path::Path;

use protobuf;
use rustc_serialize::base64::{self, ToBase64};
use sodiumoxide::crypto::sign::ed25519;

use crypto::{PublicKey, SecretKey, slice_to_sk};
use error::SimplesResult;
use simples_pb::{Wallet, WalletKeypair};

pub fn load_proto_from_file<Message: protobuf::MessageStatic>(
    path: &str) -> SimplesResult<Message>
{
    let mut proto_in = try!(File::open(&Path::new(path)));
    let mut wallet_bytes = vec![];
    try!(proto_in.read_to_end(&mut wallet_bytes));
    Ok(try!(protobuf::parse_from_bytes(&wallet_bytes)))
}

pub fn save_proto_to_file<Message: protobuf::MessageStatic>(
    path: &str, msg: &Message) -> SimplesResult<()>
{
    let mut proto_out = try!(File::create(&Path::new(path)));
    let wallet_bytes = try!(msg.write_to_bytes());
    Ok(try!(proto_out.write_all(&wallet_bytes)))
}

pub fn load_from_file(path: &str) -> SimplesResult<Wallet> {
    load_proto_from_file(path)
}

pub fn save_to_file(path: &str, wallet: &Wallet) -> SimplesResult<()> {
    save_proto_to_file(path, wallet)
}

pub fn pretty_format(wallet_key: &WalletKeypair) -> String {
    let mut formatted = String::new();
    let pk = wallet_key.get_public_key();
    let sk = wallet_key.get_secret_key();
    formatted.push_str(&format!(
        "[ {} ]\n", pk.to_base64(base64::STANDARD)));
    formatted.push_str(&format!(
        " name: {}\n", wallet_key.get_name()));
    formatted.push_str(&format!(
        "   pk: {}\n", pk.to_base64(base64::STANDARD)));
    formatted.push_str(&format!(
        "   sk: {}\n", sk.to_base64(base64::STANDARD)));
    formatted
}

pub trait WalletExt {
    fn add_key(&mut self, name: &str, public_key: &PublicKey,
               secret_key: &SecretKey);
    fn add_public_key(&mut self, name: &str, public_key: &PublicKey);
    fn generate_name(&self) -> String;
    fn generate_new_key(&mut self, name: &str) -> WalletKeypair;
    fn search_keys(&self, search_str: &str) -> Vec<&WalletKeypair>;
}

impl WalletExt for Wallet {
    fn add_key(&mut self, name: &str, public_key: &PublicKey,
               secret_key: &SecretKey) {
        let mut key = WalletKeypair::new();
        key.set_public_key(public_key.0.to_vec());
        key.set_secret_key(secret_key.0.to_vec());
        key.set_name(String::from_str(name));
        self.mut_keypairs().push(key);
    }

    fn add_public_key(&mut self, name: &str, public_key: &PublicKey) {
        let mut key = WalletKeypair::new();
        key.set_public_key(public_key.0.to_vec());
        key.set_name(String::from_str(name));
        self.mut_keypairs().push(key);
    }

    fn generate_new_key(&mut self, name: &str) -> WalletKeypair {
        let (pk, sk) = ed25519::gen_keypair();
        let mut key = WalletKeypair::new();
        key.set_public_key(pk.0.to_vec());
        key.set_secret_key(sk.0.to_vec());
        key.set_name(String::from_str(name));
        let copy = key.clone();
        self.mut_keypairs().push(key);
        copy
    }

    fn generate_name(&self) -> String {
        let already_exists = |name| -> bool {
            let dups: Vec<&WalletKeypair> =
                self.get_keypairs().iter().filter(|k| {
                    k.get_name() == name
                }).collect();
            dups.len() > 0
        };
        let make_name = |i: usize| { format!("addr{}", i) };
        make_name(count(1, 1)
                  .take_while(|i| {
                      already_exists(make_name(*i))
                  }).last().unwrap_or(0) + 1)
    }

    fn search_keys(&self, search_str: &str) -> Vec<&WalletKeypair> {
        self.get_keypairs().iter()
            .filter(|wkey| {
                let name = &wkey.get_name()[..];
                let pk_base64 = wkey.get_public_key()
                    .to_base64(base64::STANDARD);
                name.starts_with(search_str) ||
                    pk_base64.starts_with(search_str)
            }).collect()
    }
}

pub trait WalletKeypairExt {
    fn decode_public_key(&self) -> SimplesResult<PublicKey>;
    fn decode_secret_key(&self) -> SimplesResult<SecretKey>;
}

impl WalletKeypairExt for WalletKeypair {
    fn decode_public_key(&self) -> SimplesResult<PublicKey> {
        PublicKey::from_bytes(self.get_public_key())
    }

    fn decode_secret_key(&self) -> SimplesResult<SecretKey> {
        slice_to_sk(self.get_secret_key())
    }
}
