use lsp_types::notification::DidOpenTextDocument;
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::{DidOpenTextDocumentParams, DocumentDiagnosticParams, TextDocumentItem};
use std::fs;
use std::path::Path;
use test_utils::fixtures::test_state::{named, named_with_deps};

mod support;

#[test]
fn test_get_diagnostics_for_a_file() {
    let dep_path = Path::new("..").join("MyDep").to_string_lossy().to_string();
    let server = support::project(vec![
        named_with_deps(
            "MyPackage",
            // language=TOML
            &format!(
                r#"
[dependencies]
MyDep = {{ local = "{dep_path}"}}
            "#
            ),
            // language=Move
            r#"
//- /main.move
module std::m {
    use std::table::Table;
    use std::table::Unknown;
    fun main(t: Table) {
    }
}
    "#,
        ),
        named(
            "MyDep",
            // language=Move
            r#"
//- /table.move
module std::table {
    struct Table { val: u8 }
}
    "#,
        ),
    ]);
    let rel_main_document = Path::new("MyPackage")
        .join("sources")
        .join("main.move")
        .to_string_lossy()
        .to_string();
    let main_document = server.doc_id(&rel_main_document);
    let main_document_contents = fs::read_to_string(main_document.uri.path()).unwrap();
    server.notification::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem::new(
            main_document.uri.clone(),
            "move".to_string(),
            0,
            main_document_contents,
        ),
    });

    let server = server.wait_until_workspace_is_loaded();

    let resp = server.send_request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
        text_document: main_document,
        identifier: None,
        previous_result_id: None,
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    });
    let s = serde_json::to_string_pretty(&resp).unwrap();
    let expected_resp = expect_test::expect![[r#"
        {
          "kind": "full",
          "resultId": "aptos-analyzer",
          "items": [
            {
              "range": {
                "start": {
                  "line": 2,
                  "character": 20
                },
                "end": {
                  "line": 2,
                  "character": 27
                }
              },
              "severity": 1,
              "code": "unresolved-reference",
              "source": "aptos-analyzer",
              "message": "Unresolved reference `Unknown`: cannot resolve"
            }
          ]
        }"#]];
    expected_resp.assert_eq(&s);
}
