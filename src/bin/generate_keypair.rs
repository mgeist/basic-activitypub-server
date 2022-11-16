use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey, RsaPublicKey,
};

fn main() {
    // We'll use 2048 bits, same as the article uses
    let bits = 2048;

    // Generate our public and private key pair
    let private_key = RsaPrivateKey::new(&mut OsRng, bits).unwrap();
    let public_key = RsaPublicKey::from(&private_key);

    // Write the keys to disk as public.pem and private.pem respectively.
    // Note that we are explicitly using LF line endings here (\n) since we're
    // going to serve the public key file and Mastodon expects the LF line
    // endings.
    private_key
        .write_pkcs8_pem_file("private.pem", LineEnding::LF)
        .unwrap();

    public_key
        .write_public_key_pem_file("public.pem", LineEnding::LF)
        .unwrap();
}
