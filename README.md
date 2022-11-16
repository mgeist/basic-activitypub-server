# Basic ActivityPub Server (in Rust)

This is a deep-dive on this blog post: https://blog.joinmastodon.org/2018/06/how-to-implement-a-basic-activitypub-server/

We will go through all of the pieces required to successfully reply to a post on a Mastodon server, including creating and deploying a simple webserver. You are free to substitute anything here if you have preferences on things like deployment or libraries. Whatever makes you happiest. I'm going to gloss over some details, since I don't want to duplicate the blog article that this was based on, so you should read that first to get an idea for what we are going to do. In summary, we're going to write a simple web server to serve 2 endpoints: `/.well-known/webfinger` and `/actor`, create a script to generate an RSA key pair for signing our request to Mastodon, and write a small script to create a reply post to a Mastodon user.

It's also worth noting that this is not intended to be a guide on best practices. Many of these libraries are new to me, and I wanted to keep the code fairly minimal. I definitely do not recommend using this code as-is beyond demonstration and learning.

## Creating the project

First things first, we need a new Rust project. We'll be making a few different commands in `/src/bin`, and using a `lib.rs` file, but using `--lib` puts `Cargo.lock` in the `.gitignore` which is not quite what we want.

`cargo new basic-activitypub-server`

You can probably just make a `lib.rs` and delete `main.rs`, if you want to be ahead of the class.

## nix-shell

As a baby NixOS user, this is the `shell.nix` file that I am using. It includes things like `rustfmt` and `rust-analyzer` since I run my editor in the nix shell. Nix people, share with me your secrets and protips. If you aren't using Nix, this is a hint to the things you probably want installed. The only real notable one besides Rust and normal dev stuff is `flyctl` since we're going to deploy this to a free fly.io instance!


```nix
# File: shell.nix

{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell rec {
    buildInputs = with pkgs; [
        # general dev
        git

        # rust stuff
        cargo
        clippy
        rust-analyzer
        rustc
        rustfmt

        # deployment stuff
        flyctl
    ];
}
```

## Creating an RSA keypair

One of the first steps in the article is to generate a keypair using `openssl`.

Now, we _could_ install OpenSSL and create a keypair the way that article suggests, but this wouldn't be a Rust project if we did that. Most of us probably even have `openssl` installed already, but we'll ignore that for now. This one is for the rewrite-it-in-Rust folks.

We'll need to add a new dependency for this:
```toml
# File: Cargo.toml
...
[dependencies]
rsa = "0.7"
```

Generating the key pairs is pretty straight-forward using the `rsa` library. One thing worth noting is that we're explicitly using `LineEnding::LF` (`\n`) since the blog article later mentions expecting the key in that format. I don't think that should cause any issues for Windows users, besides the files displaying oddly.

We write these to the current directory, which will probably be the root of your project since that's where you'll run cargo. In a real world scenario, you want to keep the `private.pem` file safe and secure. We'll add the `.pem` files to `.gitignore` just to be sure.

```
# File: .gitignore

/target
*.pem
```

```rust
// File: src/bin/generate_keypair.rs

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

    // Write the keys to disk as private.pem and public.pem respectively.
    private_key
        .write_pkcs8_pem_file("private.pem", LineEnding::LF)
        .unwrap();

    public_key
        .write_public_key_pem_file("public.pem", LineEnding::LF)
        .unwrap();
}
```

Now if we run `cargo run --bin generate_keypair`, we should see a couple shiny new `.pem` files.

Hold on to those for now. Or don't, you can make more whenever you want.

## A web server

Ok, that was a waste of time, but I had fun. Let's get started on the web server. Part of the process of sending a post as a reply to a Mastodon user requires the "home" server verifying some information, so we'll need to be able to serve that.

So, we need a web server. I'm going to use `axum`, and the `0.6` release candidate to boot. What could go wrong?

```toml
# File: Cargo.toml
...
[dependencies]
axum = "0.6.0-rc.4"
rsa = "0.7"
tokio = { version = "1.0", features = ["full"] }
```

I have no idea if we really need `"full"` features for `tokio`, as this is my first time using `axum` or `tokio`. That's copied straight out of the `axum` examples. This all feels a bit overkill to serve a bit of JSON, but that's okay, we're on a mission and honestly `axum` seems pretty nice. 

```rust
// File: src/lib.rs

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

pub fn app() -> Router {
    Router::new().route("/", get(hello))
}

async fn hello() -> impl IntoResponse {
    Html("<h1>Hello</h1>")
}
```

Nothing too fancy here, we're creating a basic server, and serving `<h1>Hello</h1>` at the root path. Let's create a little server bin.

```rust
// File: src/bin/server.rs

use std::net::SocketAddr;

use basic_activitypub_server::app;

#[tokio::main]
async fn main() {
    let app = app();

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    println!("Server started at {}", address);

    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

Nice. Now if we run `cargo run --bin server`, it will start listening on `0.0.0.0:8080`. We can test this with `curl localhost:8080`. You should get `<h1>Hello</h1>` back. We're making progress now.

It is worth noting that running on `0.0.0.0` potentially exposes the process to your network, so if you prefer, you can use `127.0.0.1` instead for connections from this machine only. We'll need `0.0.0.0` when we deploy to fly.io, so keep that in mind if you do change it.

## Webfinger

Great, we have a server that does nothing interesting. Let's make it do something interesting!

Our first goal will be to serve a response to webfinger requests. Basically, when we initiate our request to a Mastodon server, one of the things it will do is send a webfinger request to your server to get an account identity at `/.well-known/webfinger?resource=acct:foo@bar.baz`. Since this is just a demonstration, we can just serve one webfinger document, and completely ignore the query parameter `?resource=acct:foo@bar.baz`. As a future improvement, you could check for valid accounts and serve the webfinger responses dynamically by checking a database.

As a reminder, this is the data we need to serve:
```json
{
	"subject": "acct:alice@my-example.com",

	"links": [
		{
			"rel": "self",
			"type": "application/activity+json",
			"href": "https://my-example.com/actor"
		}
	]
}
```

Now, we don't know what our domain is going to be yet. We could have created it and hard-coded it, but let's just store it as `State` on the server. We can read it in an environment variable so we can change it without having to recompile.

First, let's add an `AppState` object, which will be our `axum` server's `State`. User and domain is enough for our purpose.

```rust
// File: src/lib.rs
...
#[derive(Clone)]
pub struct AppState {
    pub user: String,
    pub domain: String,
}

pub fn app(state: AppState) -> Router<AppState> {
    Router::with_state(state).route("/", get(hello))
}
...
```

When we start our app, we will now need to provide an `AppState` to the `app` function. Time to update our server bin. Here, we're adding user and domain, which we're grabbing from environment variables. I opted to use `.unwrap()` here just so we don't accidentally start our server without setting these.

```rust
// File: src/bin/server.rs

use std::{env, net::SocketAddr};

use basic_activitypub_server::{app, AppState};

#[tokio::main]
async fn main() {
    let user = env::var("AP_USER").unwrap();
    let domain = env::var("AP_DOMAIN").unwrap();

    let app = app(AppState { user, domain });
...
```


With that out of the way, we can finally start adding our webfinger endpoint. Technically, we could just serve the document as a string already encoded as JSON, but that's no fun. Let's go big and use `serde`.

```toml
# File: Cargo.toml
...
[dependencies]
axum = "0.6.0-rc.4"
rsa = "0.7"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

The actual route handler function is pretty straight-forward. First thing we've done here is created our `WebfingerResponse` struct, which we will use to serialize into JSON for the response. We're taking in `AppState` in the function via `State` in `axum`. This gives us access to our user and domain information. The function itself is pretty simple: instantiate the structs and send them off as JSON via the `Json` function.

You'll notice we're using `https://<domain>/actor` as our `href` here. This can be anything you want it to be. If you had a server with multiple users, it would probably be something like `https://<domain>/user/<user>`.

We also tell `serde` to rename the `Kind` field to `type` when it serializes, since we can't use `type` in Rust as it is a reserved keyword.

```rust
// File: src/webfinger.rs

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct WebfingerResponse {
    subject: String,
    links: Vec<WebfingerLink>,
}

#[derive(Serialize)]
struct WebfingerLink {
    rel: String,
    #[serde(rename = "type")]
    kind: String,
    href: String,
}

pub(crate) async fn webfinger(State(state): State<AppState>) -> impl IntoResponse {
    let self_link = WebfingerLink {
        rel: "self".to_string(),
        kind: "application/activity+json".to_string(),
        href: format!("https://{}/actor", state.domain),
    };

    let webfinger_response = WebfingerResponse {
        subject: format!("acct:{}@{}", state.user, state.domain),
        links: vec![self_link],
    };

    Json(webfinger_response)
}
```

We need to add a route to use this function. Our `app` function becomes:

```rust
// File: src/lib.rs

mod webfinger; // Don't forget to add this so we can use our webfinger function
...
pub fn app(state: AppState) -> Router<AppState> {
    Router::with_state(state)
        .route("/", get(hello))
        .route("/.well-known/webfinger", get(webfinger::webfinger))
}
...
```

Time to test it out. Remember, we added those environment variables. So, unless you want to set them however you prefer, we can run the server like so: `AP_USER=foo AP_DOMAIN=bar.baz cargo run --bin server`.

By running `curl localhost:8080/.well-known/webfinger`, we should get a bunch of JSON back. Success! We're nearly there.

You get extra credit if you check out your profile's webfinger on your mastodon server, if you have one. This will give you a better idea of the information they can contain. Here's mine: https://mastodon.online/.well-known/webfinger?resource=acct:geist@mastodon.online

## Actor

We only need one more endpoint, and then we can start crafting the script to send out our Mastodon post reply. That is the `/actor` endpoint. As mentioned above, this can be anything, so long as webfinger gives the proper link.

To give you a refresher, this is the actor document that we are going to give as a response:
```json
{
	"@context": [
		"https://www.w3.org/ns/activitystreams",
		"https://w3id.org/security/v1"
	],

	"id": "https://my-example.com/actor",
	"type": "Person",
	"preferredUsername": "alice",
	"inbox": "https://my-example.com/inbox",

	"publicKey": {
		"id": "https://my-example.com/actor#main-key",
		"owner": "https://my-example.com/actor",
		"publicKeyPem": "-----BEGIN PUBLIC KEY-----...-----END PUBLIC KEY-----"
	}
}
```

Finally, those keys we generated all the way up there will come in handy.

Let's create the actor route handler function first.

Look, before we go any further, don't judge me, but we're just going to hardcode the public key here. It's getting late here and I've been typing away here for a couple hours now. Let's just keep this a secret amongst friends. Just copy and paste that `public.pem` in there. I can keep a secret.

Ok, now that we're done with that, the rest of this should look pretty close to the webfinger function. We're just creating a couple structs, and sending them away into magical `Json` land. The structs are getting a `#[serde(rename_all = "camelCase")]` so that they, well, get renamed into camelCase format.

The observant of you will notice that we're adding an inbox and a few other things. I'm not sure if they're needed, I'm just copying what the original article is using. We'll touch on that later.

```rust
// File: src/actor.rs

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::AppState;

const PUB_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANB...
...
...5QIDAQAB
-----END PUBLIC KEY-----"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActorResponse {
    #[serde(rename = "@context")]
    context: Vec<String>,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    preferred_username: String,
    inbox: String,
    public_key: ActorPublicKey,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActorPublicKey {
    id: String,
    owner: String,
    public_key_pem: String,
}

pub(crate) async fn actor(State(state): State<AppState>) -> impl IntoResponse {
    let public_key = ActorPublicKey {
        id: format!("https://{}/actor#main-key", state.domain),
        owner: format!("https://{}/actor", state.domain),
        public_key_pem: PUB_KEY.to_string(),
    };

    let actor_response = ActorResponse {
        context: vec![
            "https://www.w3.org/ns/activitystreams".to_string(),
            "https://w3id.org/security/v1".to_string(),
        ],
        id: format!("https://{}/actor", state.domain),
        kind: "Person".to_string(),
        preferred_username: state.user,
        inbox: format!("https://{}/inbox", state.domain),
        public_key,
    };

    Json(actor_response)
}

```

Alright, let's go plug this into a route and test it out.

```rust
// File: src/lib.rs

mod actor; // Don't forget to add this!
...
pub fn app(state: AppState) -> Router<AppState> {
    Router::with_state(state)
        .route("/", get(hello))
        .route("/.well-known/webfinger", get(webfinger::webfinger))
        .route("/actor", get(actor::actor))
}
...
```

Restart your server if you had it running, otherwise fire it up. Same as before: `AP_USER=foo AP_DOMAIN=bar.baz cargo run --bin server`

Now hit it and let's see the magic.

`curl localhost:8080/actor`

You should see a nice big blob of JSON with your public key and all. Awesome. Almost done.

## Deployment

Before we move on to the actual fireworks, we need to get this thing deployed somewhere. If you want to keep it simple, you can probably use `ngrok` or something to simply give you an endpoint tunnel to your local server, set your domain environment variable accordingly and call it a day.

For the rest of you, let's deploy to fly.io. It is free, within their limitations, which this app certainly is. Do note however that to deploy to fly.io, you will need to input a credit card. If you don't have one accessible or do not wish to give it to them, you will have to find an alternative, unfortunately.

I'm not affiliated with fly.io, I just found it a pretty convenient way to get things deployed for testing and playing around. I'm testing it as an alternative to `heroku` which was my go-to for these sorts of things in the past. I haven't deployed anything real or meaningful to fly.io.

First, we'll need to create a `Dockerfile` and a `.dockerignore`

```
# File: .dockerignore

/target
*.pem
```

If you don't set up the `.dockerignore`, you'll end up sending a bunch of data to the remote docker builder which is terrible. Mine was using 500MB per deploy. This might be a non-issue if you have Docker installed locally, but that's fine! It still just less bytes to shovel around.

Next, create the `Dockerfile`. This uses a 2 stage build process, so that the final docker image is around 80mb. If you use the Rust image as the deployed version, your image will be around 2gb. There are ways to strip it down even smaller, but this suits our needs just fine.

```dockerfile
# File: Dockerfile

# BUILDER
FROM rust:1.65.0 AS builder

WORKDIR /app

COPY Cargo* ./
COPY src/ src/

RUN cargo build --bin server --release

# FINAL IMAGE
FROM debian:buster-slim

ENV AP_USER=your-username-here
ENV AP_DOMAIN=your-domain-here.fly.dev

EXPOSE 8080/tcp

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server

ENTRYPOINT ["./server"]
```

But wait, we don't have a domain yet. We'll come back to that, let's start working on fly.io. Go to https://fly.io and create an account. If you need to, install `flyctl`, their CLI tool. You can find information about there here: https://fly.io/docs/hands-on/install-flyctl/.

Once you've installed `flyctl` and get signed up, run `flyctl launch`. This will prompt you for an app name, which you can opt to leave blank to get an autogenerated one.

Next, choose a region. This is personal preference, probably whatever is closest to you. This is the region in which the server will be deployed.

When it asks you if you would like to setup a postgres database, just say no. We don't need a database where we're going.

When it asks you to deploy now, say no. We still need to set the domain environment variable in our Dockerfile.

Great, now you can run `flyctl status` to get your app URL it has given you. It's listed as "Hostname". Take that, and set it in your Dockerfile: `ENV AP_DOMAIN=whatever-it-lists-here.fly.dev`

You'll also notice that a `fly.toml` file has been created for you. This is generated by `flyctl` and you can use it to further configure the fly.io app.

Once you've set the `AP_USER` and `AP_DOMAIN` environment variables, it's time to deploy. This could take up to 6 minutes, so be patient. Future deploys will be around 2-4 minutes, and it's possible to get it a lot faster if we had a more refined Dockerfile. Most of the deploy time right now is downloading the crates.io index and building all the dependencies. There's probably a fancy way to improve the caching, in which case deployment would be really fast. If any readers know of good techniques to improve cargo caching in Docker builds, I'd love to hear about them.

`flyctl deploy`

You should see `--> v0 deployed successfully`.

Check your endpoints to see them in action. You can visit `https://yourdomain.fly.dev/actor` and `https://yourdomain.fly.dev/.well-known/webfinger` and should see the correct responses.


## Post to Mastodon

It is time. After all of that work, we finally have a little app deployed which will act as our ActivityPub server for other ActivityPub instances to communicate with. It doesn't do much, but it's enough to reply to a Mastodon post.

We are going to write the equivalent of this Ruby script from the original article:
```ruby
require 'http'
require 'openssl'

document      = File.read('create-hello-world.json')
date          = Time.now.utc.httpdate
keypair       = OpenSSL::PKey::RSA.new(File.read('private.pem'))
signed_string = "(request-target): post /inbox\nhost: mastodon.social\ndate: #{date}"
signature     = Base64.strict_encode64(keypair.sign(OpenSSL::Digest::SHA256.new, signed_string))
header        = 'keyId="https://my-example.com/actor",headers="(request-target) host date",signature="' + signature + '"'

HTTP.headers({ 'Host': 'mastodon.social', 'Date': date, 'Signature': header })
    .post('https://mastodon.social/inbox', body: document)
```

We need to add a few dependencies before we go any further.

```toml
# File: Cargo.toml

[dependencies]
axum = "0.6.0-rc.4"
base64 = "0.13"
reqwest = { version = "0.11", default-features = false, features = ["rustls-tls"] }
rsa = "0.7"
serde = { version = "1.0", features = ["derive"] }
sha2 = { version = "0.10", features = ["oid"] }
time = { version = "0.3", features = ["std", "formatting"] }
tokio = { version = "1.0", features = ["full"] }
```

We've added the following dependencies:
- `base64` which encodes text into base64 format
- `reqwest` is a library for making http requests. It is based on the same underlying library as `axum`
- `sha2` for all of our `Sha256` hashing needs
- `time` to give us our fancily-formatted UTC datetime

Let's create a new bin file which will be our `reply` action.

We're taking a lot of shortcuts by hardcoding things, but hey, the Ruby script is like 10 lines long. I wanted to be at least on the same order of magnitude. 

It's best to look at this in two parts. First, the `CREATE_REPLY` string. Check this carefully, you will need to update any of the fly.dev domains with your own. The `inReplyTo` is the post receiving the reply. This is set to my first Mastodon post, you're welcome to keep it, or switch it for your own post. The `content` field is the actual content of the message. Set it to whatever you like. You'll also notice routes which do not exist: `/create-hello-world` and `/hello-world`. It doesn't seem to matter that these don't exist for this. I'm sure the ActivityPub spec has more details about that.

Ok, now on to the Rust bits. Make sure you have the `private.pem` file in your project root folder. Note that this MUST be the one associated with the public key that you set in your `actor.rs` file and have deployed to your server.

Next is the `date`, I don't know why Ruby / Mastodon are using a different time spec, but I couldn't find a quick easy Rust library to do it directly, so we're going to hack the offset to instead be `GMT` which is what Mastodon wants.

The `digest` is a Sha256 hash of the `CREATE_REPLY` document in the form of `SHA-256=<base64-encoded-hash>`

The `string_to_sign` is the actual string that gets signed, like it says on the tin. This will be recreated by Mastodon from the request headers, so be sure there aren't any typos here. Note that the `host` must match the domain in `inReplyTo` from the `CREATE_REPLY`, and anywhere else this example uses `mastodon.online` should match as well.

The string is signed and then base64 encoded to be used in the `signature_header`. The `keyId` field in the `signature_header` must point to your public key. The `#main-key` bit is not really necessary, so long as it matches the id of the public key in the `actor` document, so you can omit that in a later version if you like.

```rust
// File: src/bin/reply.rs

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
		"content": "<p>Hello from me</p>",
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
```

Finally, it is time to send the request to our Mastodon server of choice. 

`cargo run --bin reply`

If you've gotten everything correct, you should see a status code of 2xx. If you get a different status code, the `Response text:` should give you some error.

I don't think these give notifications by default, and my second test example hasn't shown up, which may be due to Mastodon thinking I'm spamming by posting from three different accounts or something. But yours will likely show up if you see a 2xx HTTP status code in your response.

And that's it!

## Where to go from here

Remember the `/inbox` route I mentioned earlier? You can try hooking up a handler to receive `POST` requests on it. I seem to get delete actions from the mastodon instance I interacted with, even on things I've never seen. You can also hook up some basic logging to see what kind of requests your server will start receiving, if you're curious.

Beyond that, if you want to continue exploring, it's probably time to start learning more about the ActivityPub and Webfinger specifications. Happy hacking out there.