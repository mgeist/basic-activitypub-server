use rsa::{pkcs1v15::SigningKey, pkcs8::DecodePrivateKey, signature::Signer, RsaPrivateKey};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc2822, OffsetDateTime};

const CREATE_REPLY: &str = r#"
    "@context": "https://www.w3.org/ns/activitystreams",
	"id": "https://wispy-violet-1010.fly.dev/create-hello-world",
	"type": "Create",
	"actor": "https://wispy-violet-1010.fly.dev/actor",
	"object": {
		"id": "https://wispy-violet-1010.fly.dev/hello-world",
		"type": "Note",
		"published": "2022-15-11T05:09:59Z",
		"attributedTo": "https://wispy-violet-1010.fly.dev/actor",
		"inReplyTo": "https://mastodon.online/@geist/109266692665758321",
		"content": "<p>Hello from @geist@mastodon.online</p>",
		"to": "https://www.w3.org/ns/activitystreams#Public"
	}
"#;

#[tokio::main]
async fn main() {
    let private_key = RsaPrivateKey::read_pkcs8_pem_file("private.pem").unwrap();
    let signing_key = SigningKey::<Sha256>::new_with_prefix(private_key);

    let date = OffsetDateTime::now_utc().format(&Rfc2822).unwrap();
    let date = date.replace("+0000", "GMT");

    let digest = base64::encode(Sha256::digest(CREATE_REPLY));
    let digest = format!("SHA-256={}", digest);

    let string_to_sign = format!(
        "(request-target): post /inbox\nhost: mastodon.online\ndate: {}\ndigest: {}",
        date, digest
    );
    let signature = base64::encode(signing_key.sign(string_to_sign.as_bytes()));

    let key_id = "https://wispy-violet-1010.fly.dev/actor#main-key";
    let signature_header = format!(
        r#"keyId="{}",headers="(request-target) host date digest",signature="{}""#,
        key_id, signature
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://mastodon.online/inbox")
        .header("Host", "mastodon.online")
        .header("Date", date)
        .header("Digest", digest)
        .header("Signature", signature_header)
        .body(CREATE_REPLY)
        .send()
        .await
        .unwrap();
    println!("Status code: {}", response.status());
    println!("Response text: {}", response.text().await.unwrap());
}
