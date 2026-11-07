//! Loads a model file and writes a few words with it: run by hand before adding a model to the
//! catalogue, since a file the inference library cannot load would fail every player who picks
//! it.
//!
//! `MRGLGLGL_MODEL_FILE=/path/model.gguf cargo test -p mrglglgl-llm --features local --test \
//! local_load -- --ignored --nocapture`
#![cfg(feature = "local")]

use std::time::Duration;

use mrglglgl_llm::local::{LocalOptions, LocalProvider};
use mrglglgl_llm::{CancellationToken, GenerateOutcome, GenerateRequest, generate_text};

#[tokio::test]
#[ignore = "loads a real model file; needs MRGLGLGL_MODEL_FILE"]
async fn a_model_file_loads_and_writes() {
    let path = std::env::var("MRGLGLGL_MODEL_FILE").expect("MRGLGLGL_MODEL_FILE");
    let reasoning = std::env::var("MRGLGLGL_MODEL_REASONING").is_ok_and(|v| v == "1");
    let provider = LocalProvider::new(LocalOptions::new(&path, "under-test", reasoning));
    let request = GenerateRequest {
        system: "You are the murloc scribe of a World of Warcraft adventurer. Write one short \
                 journal entry in the first person."
            .into(),
        user: "Notes: caught 6 fish at Stonebull Lake. Reached level 10.".into(),
        max_tokens: 120,
        temperature: 0.4,
        language: "en".into(),
    };
    let (outcome, text) = generate_text(
        &provider,
        request,
        Duration::from_secs(600),
        CancellationToken::new(),
        None,
    )
    .await
    .expect("the model loads and generates");
    println!("--- {path}\n{text}");
    assert_eq!(outcome, GenerateOutcome::Completed);
    assert!(text.split_whitespace().count() > 10, "{text}");
}
