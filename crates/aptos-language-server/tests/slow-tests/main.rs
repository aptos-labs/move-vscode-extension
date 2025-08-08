// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use lsp_types::notification::DidOpenTextDocument;
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::{DidOpenTextDocumentParams, DocumentDiagnosticParams, TextDocumentItem};
use std::fs;
use test_utils::fixtures::test_state::{named, named_with_deps};

mod support;

#[test]
fn test_get_diagnostics_for_a_file() {
    let server = support::project(vec![
        named_with_deps(
            "MyPackage",
            // language=TOML
            r#"
[dependencies]
MyDep = { local = "../MyDep"}
            "#,
            // language=Move
            r#"
//- /main.move
module std::m {
    use std::table::Table;
    use std::table::Unknown;
    fun main(_t: Table) {
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
    let main_document = server.doc_id("MyPackage/sources/main.move");
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
          "resultId": "aptos-language-server",
          "items": [
            {
              "range": {
                "start": {
                  "line": 2,
                  "character": 4
                },
                "end": {
                  "line": 2,
                  "character": 28
                }
              },
              "severity": 2,
              "code": "unused-import",
              "source": "aptos-language-server",
              "message": "Unused use item"
            },
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
              "source": "aptos-language-server",
              "message": "Unresolved reference `Unknown`: cannot resolve"
            }
          ]
        }"#]];
    expected_resp.assert_eq(&s);
}
