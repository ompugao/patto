mod common;

use common::*;

#[tokio::test]
async fn test_lsp_codec() {
    use serde_json::json;
    use tokio_util::bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};

    let mut codec = LspCodec;
    let mut buf = BytesMut::new();

    let msg = json!({
        "jsonrpc": "2.0",
        "method": "testMethod",
        "params": {
            "key": "value"
        }
    });

    codec.encode(msg.clone(), &mut buf).unwrap();
    let decoded = codec.decode(&mut buf).unwrap().unwrap();

    assert_eq!(decoded, msg);
}

#[tokio::test]
async fn test_workspace() {
    let mut workspace = TestWorkspace::new();
    let _path = workspace.create_file("test.pn", "content\n");

    let uri = workspace.get_uri("test.pn");
    assert!(uri.as_str().contains("test.pn"));
}

#[tokio::test]
#[ignore] // Requires patto-lsp to be installed
async fn test_lsp_client_initialization() {
    let workspace = TestWorkspace::new();
    let mut client = LspTestClient::new(&workspace).await;

    let response = client.initialize().await;

    assert!(response.get("result").is_some());
    assert!(response["result"]["capabilities"].is_object());
}
